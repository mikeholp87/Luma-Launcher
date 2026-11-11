use std::collections::BTreeMap;

/// Search keyword to URL template, where `%s` stands for the search terms.
pub type Engines = BTreeMap<String, String>;
pub fn default_engines() -> Engines {
    [
        ("g", "https://www.google.com/search?q=%s"),
        ("ddg", "https://duckduckgo.com/?q=%s"),
    ]
    .into_iter()
    .map(|(k, v)| (k.into(), v.into()))
    .collect()
}
/// One `keyword URL` pair per line, as edited in Settings.
pub fn engines_to_text(engines: &Engines) -> String {
    engines
        .iter()
        .map(|(keyword, template)| format!("{keyword} {template}\n"))
        .collect()
}
pub fn engines_from_text(text: &str) -> Result<Engines, String> {
    let mut engines = Engines::new();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let (keyword, template) = line
            .split_once(char::is_whitespace)
            .ok_or_else(|| format!("\"{line}\" needs a keyword and a URL"))?;
        if engines
            .insert(keyword.into(), template.trim().into())
            .is_some()
        {
            return Err(format!("\"{keyword}\" is listed twice"));
        }
    }
    Ok(engines)
}
pub fn is_application_query(query: &str, engines: &Engines) -> bool {
    !query.trim_start().starts_with('=') && web(query.trim(), engines).is_none()
}
/// Terms are percent-encoded with `%20` for spaces, so templates may place
/// them in a path (`/wiki/%s`) as well as a query string.
pub fn search_url(template: &str, terms: &str) -> Option<String> {
    let mut encoded = String::new();
    for b in terms.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            encoded.push(b as char);
        } else {
            encoded.push_str(&format!("%{b:02X}"));
        }
    }
    let u = url::Url::parse(&template.replace("%s", &encoded)).ok()?;
    (matches!(u.scheme(), "http" | "https") && u.host_str().is_some()).then(|| u.into())
}
pub fn web(query: &str, engines: &Engines) -> Option<String> {
    if let Some((keyword, terms)) = query.split_once(char::is_whitespace) {
        let terms = terms.trim();
        if let Some(template) = engines.get(keyword).filter(|_| !terms.is_empty()) {
            return search_url(template, terms);
        }
    }
    // `example.com:8080` parses as a URL with scheme `example.com`.
    let u = match url::Url::parse(query) {
        Ok(u) if matches!(u.scheme(), "http" | "https") => u,
        _ => url::Url::parse(&format!("{}://{query}", bare_address(query)?)).ok()?,
    };
    (u.host_str().is_some() && u.username().is_empty() && u.password().is_none()).then(|| u.into())
}
/// Scheme for `github.com/rust-lang` or `localhost:3000` style input. Only
/// well-known TLDs qualify so app IDs like `org.gnome.Nautilus` and file
/// names stay searches; local development servers get plain HTTP.
fn bare_address(query: &str) -> Option<&'static str> {
    const TLDS: [&str; 9] = ["com", "org", "net", "io", "dev", "app", "ai", "edu", "gov"];
    let authority = query.split(['/', '?', '#']).next().unwrap_or_default();
    let (host, port) = match authority.split_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    };
    let valid_port = |p: &str| (1..=5).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_digit());
    if !host.is_ascii() || host.contains(['@', ' ']) || port.is_some_and(|p| !valid_port(p)) {
        return None;
    }
    if host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1" {
        return port.is_some().then_some("http");
    }
    host.rsplit_once('.')
        .is_some_and(|(name, tld)| {
            !name.is_empty() && TLDS.contains(&tld.to_ascii_lowercase().as_str())
        })
        .then_some("https")
}
/// Human-readable result: no float noise (`0.1+0.2` is `0.3`) and no
/// exponent-free megastrings for huge values.
pub fn format_number(v: f64) -> String {
    if v.abs() >= 1e15 {
        return format!("{v:e}");
    }
    let s = format!("{v:.10}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0" } else { s }.to_string()
}
/// Calculator text for the expression after `=`, including an optional
/// `in hex`, `in bin` or `in dec` suffix.
pub fn evaluate(s: &str) -> Result<String, &'static str> {
    let (expr, base) = match s.trim_end().rsplit_once(" in ") {
        Some((expr, base)) if ["hex", "bin", "dec"].contains(&base.trim()) => (expr, base.trim()),
        _ => (s, "dec"),
    };
    let v = calculate(expr)?;
    if base == "dec" {
        return Ok(format_number(v));
    }
    if v.fract() != 0.0 || v.abs() > 9_007_199_254_740_992.0 {
        return Err("Only whole numbers convert to hex or binary");
    }
    let (sign, n) = (if v < 0.0 { "-" } else { "" }, v.abs() as u64);
    Ok(match base {
        "hex" => format!("{sign}0x{n:x}"),
        _ => format!("{sign}0b{n:b}"),
    })
}
/// `+ - * / ^`, postfix `%`, parentheses, `0x`/`0b` literals, `pi`, `e` and
/// a few functions. Parsed by hand; nothing is ever executed.
pub fn calculate(s: &str) -> Result<f64, &'static str> {
    if s.len() > 256 || !s.is_ascii() {
        return Err("Expression too long or invalid");
    }
    struct Parser<'a> {
        s: &'a [u8],
        i: usize,
    }
    impl Parser<'_> {
        fn ws(&mut self) {
            while self.s.get(self.i).is_some_and(u8::is_ascii_whitespace) {
                self.i += 1
            }
        }
        fn eat(&mut self, c: u8) -> bool {
            self.ws();
            let found = self.s.get(self.i) == Some(&c);
            self.i += usize::from(found);
            found
        }
        /// `200 + 10%` adds ten percent of 200, as on a phone calculator.
        fn expr(&mut self, depth: usize) -> Result<f64, &'static str> {
            let mut v = self.term(depth)?.0;
            loop {
                let sign = if self.eat(b'+') {
                    1.0
                } else if self.eat(b'-') {
                    -1.0
                } else {
                    return Ok(v);
                };
                let (t, percent) = self.term(depth)?;
                v += sign * if percent { v * t } else { t };
            }
        }
        /// The value, and whether it is a lone percentage such as `10%`.
        fn term(&mut self, depth: usize) -> Result<(f64, bool), &'static str> {
            let (mut v, mut percent) = self.factor(depth)?;
            loop {
                if self.eat(b'*') {
                    v *= self.factor(depth)?.0;
                } else if self.eat(b'/') {
                    let d = self.factor(depth)?.0;
                    if d == 0.0 {
                        return Err("Division by zero");
                    };
                    v /= d
                } else {
                    return Ok((v, percent));
                }
                percent = false;
            }
        }
        fn factor(&mut self, depth: usize) -> Result<(f64, bool), &'static str> {
            let v = self.unary(depth)?;
            Ok(if self.eat(b'%') {
                (v / 100.0, true)
            } else {
                (v, false)
            })
        }
        /// Signs bind looser than `^`, so `-2^2` is -4.
        fn unary(&mut self, depth: usize) -> Result<f64, &'static str> {
            if depth > 24 {
                return Err("Too many parentheses");
            };
            if self.eat(b'-') {
                Ok(-self.unary(depth + 1)?)
            } else if self.eat(b'+') {
                self.unary(depth + 1)
            } else {
                self.power(depth)
            }
        }
        /// Right-associative: `2^3^2` is 2^9.
        fn power(&mut self, depth: usize) -> Result<f64, &'static str> {
            let base = self.atom(depth)?;
            if self.eat(b'^') {
                Ok(base.powf(self.unary(depth + 1)?))
            } else {
                Ok(base)
            }
        }
        fn atom(&mut self, depth: usize) -> Result<f64, &'static str> {
            if self.eat(b'(') {
                let v = self.expr(depth + 1)?;
                if !self.eat(b')') {
                    return Err("Missing closing parenthesis");
                };
                return Ok(v);
            }
            let start = self.i;
            if self.s.get(self.i).is_some_and(u8::is_ascii_alphabetic) {
                while self.s.get(self.i).is_some_and(u8::is_ascii_alphabetic) {
                    self.i += 1
                }
                let name = std::str::from_utf8(&self.s[start..self.i]).unwrap_or("");
                return match name {
                    "pi" => Ok(std::f64::consts::PI),
                    "e" => Ok(std::f64::consts::E),
                    _ => {
                        let f: fn(f64) -> f64 = match name {
                            "sqrt" => f64::sqrt,
                            "abs" => f64::abs,
                            "round" => f64::round,
                            "floor" => f64::floor,
                            "ceil" => f64::ceil,
                            "ln" => f64::ln,
                            "log" => f64::log10,
                            "sin" => f64::sin,
                            "cos" => f64::cos,
                            "tan" => f64::tan,
                            _ => return Err("Unknown name"),
                        };
                        if !self.eat(b'(') {
                            return Err("Functions need parentheses, such as sqrt(2)");
                        }
                        let v = self.expr(depth + 1)?;
                        if !self.eat(b')') {
                            return Err("Missing closing parenthesis");
                        };
                        Ok(f(v))
                    }
                };
            }
            let radix = match self.s.get(self.i..self.i + 2) {
                Some(b"0x" | b"0X") => 16,
                Some(b"0b" | b"0B") => 2,
                _ => 10,
            };
            if radix != 10 {
                self.i += 2;
                let digits = self.i;
                while self.s.get(self.i).is_some_and(u8::is_ascii_hexdigit) {
                    self.i += 1
                }
                let text = std::str::from_utf8(&self.s[digits..self.i]).unwrap_or("");
                return u64::from_str_radix(text, radix)
                    .map(|n| n as f64)
                    .map_err(|_| "Expected a number");
            }
            while self
                .s
                .get(self.i)
                .is_some_and(|c| c.is_ascii_digit() || *c == b'.')
            {
                self.i += 1
            }
            std::str::from_utf8(&self.s[start..self.i])
                .unwrap_or("")
                .parse()
                .map_err(|_| "Expected a number")
        }
    }
    let mut p = Parser {
        s: s.trim().as_bytes(),
        i: 0,
    };
    let v = p.expr(0)?;
    p.ws();
    if p.i != p.s.len() {
        Err("Invalid expression")
    } else if !v.is_finite() {
        Err("Result is undefined")
    } else {
        Ok(v)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn web(q: &str) -> Option<String> {
        super::web(q, &default_engines())
    }
    fn is_application_query(q: &str) -> bool {
        super::is_application_query(q, &default_engines())
    }
    #[test]
    fn calculator() {
        assert_eq!(calculate("(250 + 75) * 2"), Ok(650.));
        for s in ["1/0", "system('ls')", "1 2", "(", "NaN", "2 ^", "5 % 3"] {
            assert!(calculate(s).is_err(), "{s}")
        }
    }
    #[test]
    fn powers_bind_tighter_than_signs_and_associate_right() {
        for (s, expected) in [
            ("2^10", 1024.),
            ("-2^2", -4.),
            ("2^3^2", 512.),
            ("2^-1", 0.5),
            ("(1 + 1) ^ 3 * 2", 16.),
        ] {
            assert_eq!(calculate(s), Ok(expected), "{s}");
        }
    }
    #[test]
    fn percentages_work_like_a_phone_calculator() {
        for (s, expected) in [
            ("50%", 0.5),
            ("200 + 10%", 220.),
            ("200 - 10%", 180.),
            ("200 * 10%", 20.),
            ("80 + 50% + 50%", 180.),
        ] {
            assert_eq!(calculate(s), Ok(expected), "{s}");
        }
    }
    #[test]
    fn functions_constants_and_literals() {
        for (s, expected) in [
            ("sqrt(16) + abs(-2)", 6.),
            ("round(2.5) + floor(1.9) + ceil(1.1)", 6.),
            ("log(1000) + ln(e)", 4.),
            ("cos(0) + sin(0)", 1.),
            ("0xff + 0b101", 260.),
        ] {
            assert_eq!(calculate(s), Ok(expected), "{s}");
        }
        assert!((calculate("2 * pi").unwrap() - std::f64::consts::TAU).abs() < 1e-12);
        assert_eq!(
            calculate("sqrt 4"),
            Err("Functions need parentheses, such as sqrt(2)")
        );
        assert_eq!(calculate("foo(1)"), Err("Unknown name"));
        assert_eq!(calculate("sqrt(-1)"), Err("Result is undefined"));
        assert!(calculate("0b12").is_err());
    }
    #[test]
    fn results_convert_to_hex_and_binary() {
        assert_eq!(evaluate(" 255 in hex"), Ok("0xff".into()));
        assert_eq!(evaluate("-10 in bin"), Ok("-0b1010".into()));
        assert_eq!(evaluate("0xff in dec"), Ok("255".into()));
        assert_eq!(evaluate("0.1 + 0.2"), Ok("0.3".into()));
        assert!(evaluate("1.5 in hex").is_err());
        assert!(evaluate("2 in base64").is_err());
    }
    #[test]
    fn urls() {
        assert!(web("https://example.com/a").is_some());
        assert!(web("javascript:alert(1)").is_none());
        assert!(web("https://user:pass@example.com").is_none());
        assert_eq!(
            web("ddg rust & gtk").unwrap(),
            "https://duckduckgo.com/?q=rust%20%26%20gtk"
        );
    }
    #[test]
    fn custom_keywords_fill_their_template() {
        let mut engines = default_engines();
        engines.insert("w".into(), "https://en.wikipedia.org/wiki/%s".into());
        assert_eq!(
            super::web("w Rust (language)", &engines).unwrap(),
            "https://en.wikipedia.org/wiki/Rust%20%28language%29"
        );
        assert_eq!(
            super::web("g  a/b?c#d ", &engines).unwrap(),
            "https://www.google.com/search?q=a%2Fb%3Fc%23d"
        );
        engines.remove("g");
        assert!(super::web("g linux", &engines).is_none());
        assert!(super::web("w", &engines).is_none());
        assert!(super::web("w   ", &engines).is_none());
        assert!(search_url("javascript:%s", "alert(1)").is_none());
    }
    #[test]
    fn keyword_text_round_trips() {
        let engines = default_engines();
        assert_eq!(engines_from_text(&engines_to_text(&engines)), Ok(engines));
        assert_eq!(
            engines_from_text("\n  gh\thttps://github.com/search?q=%s  \n\n")
                .unwrap()
                .get("gh")
                .map(String::as_str),
            Some("https://github.com/search?q=%s")
        );
        assert!(engines_from_text("gh").is_err());
        assert!(engines_from_text("g https://a.com/%s\ng https://b.com/%s").is_err());
        assert_eq!(engines_from_text(""), Ok(Engines::new()));
    }

    #[test]
    fn bare_domains_open_but_app_ids_do_not() {
        assert_eq!(
            web("github.com/rust-lang").unwrap(),
            "https://github.com/rust-lang"
        );
        assert!(web("example.org").is_some());
        for q in [
            "org.gnome.Nautilus",
            "notes.md",
            "firefox",
            "a b.com",
            "user@site.com",
        ] {
            assert!(web(q).is_none(), "{q}");
        }
    }
    #[test]
    fn host_and_port_open_without_a_scheme() {
        assert_eq!(
            web("example.com:8080/a").unwrap(),
            "https://example.com:8080/a"
        );
        assert_eq!(web("localhost:3000").unwrap(), "http://localhost:3000/");
        assert_eq!(web("127.0.0.1:8000/x").unwrap(), "http://127.0.0.1:8000/x");
        for q in [
            "localhost",
            "javascript:alert(1)",
            "mailto:me@example.com",
            "ftp://example.com",
            "example.com:port",
            "example.com:123456",
            "notes.md:12",
        ] {
            assert!(web(q).is_none(), "{q}");
        }
    }
    #[test]
    fn numbers_format_without_float_noise() {
        for (v, expected) in [
            (0.1 + 0.2, "0.3"),
            (100., "100"),
            (1. / 3., "0.3333333333"),
            (-0., "0"),
            (-2.5, "-2.5"),
            (1e20, "1e20"),
        ] {
            assert_eq!(format_number(v), expected);
        }
    }
    #[test]
    fn tool_queries_do_not_select_applications() {
        for q in ["= 2 + 2", " = invalid", "g linux", "https://example.com"] {
            assert!(!is_application_query(q));
        }
        assert!(is_application_query("Calculator"));
    }
    #[test]
    fn calculator_accepts_ascii_whitespace() {
        assert_eq!(calculate("25\t*\n4"), Ok(100.));
    }
}
