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
    let u = url::Url::parse(query).ok()?;
    (matches!(u.scheme(), "http" | "https")
        && u.host_str().is_some()
        && u.username().is_empty()
        && u.password().is_none())
    .then(|| u.into())
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
