pub fn is_application_query(query: &str) -> bool {
    !query.trim_start().starts_with('=') && web(query.trim()).is_none()
}
pub fn web(query: &str) -> Option<String> {
    for (prefix, base) in [
        ("g ", "https://www.google.com/search"),
        ("ddg ", "https://duckduckgo.com/"),
    ] {
        if let Some(q) = query.strip_prefix(prefix).filter(|q| !q.trim().is_empty()) {
            let mut u = url::Url::parse(base).ok()?;
            u.query_pairs_mut().append_pair("q", q.trim());
            return Some(u.into());
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
        fn expr(&mut self, depth: usize) -> Result<f64, &'static str> {
            let mut v = self.term(depth)?;
            loop {
                self.ws();
                match self.s.get(self.i) {
                    Some(b'+') => {
                        self.i += 1;
                        v += self.term(depth)?
                    }
                    Some(b'-') => {
                        self.i += 1;
                        v -= self.term(depth)?
                    }
                    _ => return Ok(v),
                }
            }
        }
        fn term(&mut self, depth: usize) -> Result<f64, &'static str> {
            let mut v = self.atom(depth)?;
            loop {
                self.ws();
                match self.s.get(self.i) {
                    Some(b'*') => {
                        self.i += 1;
                        v *= self.atom(depth)?
                    }
                    Some(b'/') => {
                        self.i += 1;
                        let d = self.atom(depth)?;
                        if d == 0.0 {
                            return Err("Division by zero");
                        };
                        v /= d
                    }
                    _ => return Ok(v),
                }
            }
        }
        fn atom(&mut self, depth: usize) -> Result<f64, &'static str> {
            if depth > 24 {
                return Err("Too many parentheses");
            };
            self.ws();
            match self.s.get(self.i) {
                Some(b'-') => {
                    self.i += 1;
                    Ok(-self.atom(depth + 1)?)
                }
                Some(b'+') => {
                    self.i += 1;
                    self.atom(depth + 1)
                }
                Some(b'(') => {
                    self.i += 1;
                    let v = self.expr(depth + 1)?;
                    self.ws();
                    if self.s.get(self.i) != Some(&b')') {
                        return Err("Missing closing parenthesis");
                    };
                    self.i += 1;
                    Ok(v)
                }
                _ => {
                    let start = self.i;
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
        }
    }
    let mut p = Parser {
        s: s.trim().as_bytes(),
        i: 0,
    };
    let v = p.expr(0)?;
    p.ws();
    if p.i != p.s.len() || !v.is_finite() {
        Err("Invalid expression")
    } else {
        Ok(v)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn calculator() {
        assert_eq!(calculate("(250 + 75) * 2"), Ok(650.));
        for s in ["1/0", "system('ls')", "1 2", "(", "NaN"] {
            assert!(calculate(s).is_err())
        }
    }
    #[test]
    fn urls() {
        assert!(web("https://example.com/a").is_some());
        assert!(web("javascript:alert(1)").is_none());
        assert!(web("https://user:pass@example.com").is_none());
        assert!(web("ddg rust & gtk").unwrap().contains("rust+%26+gtk"));
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
