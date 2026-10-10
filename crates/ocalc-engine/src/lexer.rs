//! Lexer: source text → [`Token`] stream.
//!
//! Supported literals:
//! - Decimal ints/floats: `42`, `3.14`, `1e-3`, `1_000`
//! - Radix ints: `0xFF`, `0o17`, `0b1010` (underscores allowed)
//! - Identifiers: `[A-Za-z_][A-Za-z0-9_]*` (plus Greek `π` → `pi` aliasing
//!   is handled at eval time, not here)
//! - Operators: `+ - * / % ^ ** ! & | ~ << >> ( ) , =`
//! - Keywords (case-insensitive): `AND OR XOR NOT MOD`

use ocalc_number::Number;

use crate::{CalcError, Span, Token, TokenKind};

/// Lexer over a borrowed source string.
#[derive(Debug)]
pub struct Lexer<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    /// Create a lexer for `src`.
    #[must_use]
    pub fn new(src: &'a str) -> Self {
        Self {
            src,
            bytes: src.as_bytes(),
            pos: 0,
        }
    }

    /// Tokenise the whole input (including trailing [`TokenKind::Eof`]).
    ///
    /// # Errors
    /// [`CalcError::InvalidToken`] on bad characters or malformed numbers.
    pub fn tokenize(&mut self) -> Result<Vec<Token>, CalcError> {
        self.tokenize_limited(&crate::EngineLimits::default())
    }

    /// Tokenise with explicit resource limits (Phase 3 hardening).
    ///
    /// Enforces `max_expression_len` up front and `max_tokens` during the
    /// scan so hostile inputs fail fast instead of allocating unbounded
    /// vectors.
    ///
    /// # Errors
    /// [`CalcError::LimitExceeded`] when a limit is violated, else
    /// [`CalcError::InvalidToken`].
    pub fn tokenize_limited(
        &mut self,
        limits: &crate::EngineLimits,
    ) -> Result<Vec<Token>, CalcError> {
        limits.check_source_len(self.src)?;
        // Heuristic pre-reserve: ~1 token per 3 bytes avoids regrowth for
        // typical expressions without over-allocating on hostile inputs.
        let cap = (self.src.len() / 3).clamp(8, limits.max_tokens.min(8192));
        let mut out = Vec::with_capacity(cap);
        loop {
            self.skip_ws();
            if self.pos >= self.bytes.len() {
                out.push(Token::new(TokenKind::Eof, Span::point(self.pos)));
                limits.check_token_count(out.len())?;
                return Ok(out);
            }
            // Fail fast before pushing the next token.
            if out.len() + 1 >= limits.max_tokens {
                return Err(CalcError::LimitExceeded {
                    message: format!("expression exceeds token limit {}", limits.max_tokens),
                    span: Some(Span::point(self.pos)),
                });
            }
            let tok = self.next_token()?;
            let eof = matches!(tok.kind, TokenKind::Eof);
            out.push(tok);
            if eof {
                limits.check_token_count(out.len())?;
                return Ok(out);
            }
        }
    }

    // --------------------------------------------------------------

    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<u8> {
        self.bytes.get(self.pos + 1).copied()
    }

    fn next_token(&mut self) -> Result<Token, CalcError> {
        let start = self.pos;
        let b = self.peek().expect("caller checks bounds");
        match b {
            b'0'..=b'9' | b'.' => self.lex_number(),
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => Ok(self.lex_ident_or_keyword()),
            b'+' => {
                self.pos += 1;
                Ok(Token::new(TokenKind::Plus, Span::new(start, self.pos)))
            }
            b'-' => {
                self.pos += 1;
                Ok(Token::new(TokenKind::Minus, Span::new(start, self.pos)))
            }
            b'*' => {
                self.pos += 1;
                if self.peek() == Some(b'*') {
                    self.pos += 1;
                }
                Ok(Token::new(TokenKind::Star, Span::new(start, self.pos)))
                    // `**` folds into power at parse time; keep span wide.
                    .map(|mut t| {
                        if self.pos - start == 2 {
                            t.kind = TokenKind::Caret;
                        }
                        t
                    })
            }
            b'/' => {
                self.pos += 1;
                Ok(Token::new(TokenKind::Slash, Span::new(start, self.pos)))
            }
            b'%' => {
                self.pos += 1;
                Ok(Token::new(TokenKind::Percent, Span::new(start, self.pos)))
            }
            b'^' => {
                self.pos += 1;
                Ok(Token::new(TokenKind::Caret, Span::new(start, self.pos)))
            }
            b'!' => {
                self.pos += 1;
                Ok(Token::new(TokenKind::Bang, Span::new(start, self.pos)))
            }
            b'&' => {
                self.pos += 1;
                Ok(Token::new(TokenKind::Amp, Span::new(start, self.pos)))
            }
            b'|' => {
                self.pos += 1;
                Ok(Token::new(TokenKind::Pipe, Span::new(start, self.pos)))
            }
            b'~' => {
                self.pos += 1;
                Ok(Token::new(TokenKind::Tilde, Span::new(start, self.pos)))
            }
            b'(' => {
                self.pos += 1;
                Ok(Token::new(TokenKind::LParen, Span::new(start, self.pos)))
            }
            b')' => {
                self.pos += 1;
                Ok(Token::new(TokenKind::RParen, Span::new(start, self.pos)))
            }
            b',' => {
                self.pos += 1;
                Ok(Token::new(TokenKind::Comma, Span::new(start, self.pos)))
            }
            b'=' => {
                self.pos += 1;
                Ok(Token::new(TokenKind::Assign, Span::new(start, self.pos)))
            }
            b'<' => {
                if self.peek2() == Some(b'<') {
                    self.pos += 2;
                    Ok(Token::new(TokenKind::Shl, Span::new(start, self.pos)))
                } else {
                    self.pos += 1;
                    Err(CalcError::invalid_token(
                        "<".to_owned(),
                        Span::new(start, self.pos),
                        "did you mean '<<' (shift)?",
                    ))
                }
            }
            b'>' => {
                if self.peek2() == Some(b'>') {
                    self.pos += 2;
                    Ok(Token::new(TokenKind::Shr, Span::new(start, self.pos)))
                } else {
                    self.pos += 1;
                    Err(CalcError::invalid_token(
                        ">".to_owned(),
                        Span::new(start, self.pos),
                        "did you mean '>>' (shift)?",
                    ))
                }
            }
            _ => {
                // Accept non-ASCII identifier starts (e.g. π, τ) as idents so
                // `π * 2` lexes instead of failing. Anything else is invalid.
                let ch = self.src[start..].chars().next().expect("non-empty");
                if ch.is_alphabetic() {
                    Ok(self.lex_ident_or_keyword())
                } else {
                    self.pos += ch.len_utf8();
                    Err(CalcError::invalid_token(
                        ch.to_string(),
                        Span::new(start, self.pos),
                        format!("unexpected character {ch:?}"),
                    ))
                }
            }
        }
    }

    fn lex_ident_or_keyword(&mut self) -> Token {
        let start = self.pos;
        while self.pos < self.bytes.len() {
            let ch = self.src[self.pos..].chars().next().expect("char boundary");
            if ch.is_alphanumeric() || ch == '_' {
                self.pos += ch.len_utf8();
            } else {
                break;
            }
        }
        let word = &self.src[start..self.pos];
        let upper: String = word.to_ascii_uppercase();
        let span = Span::new(start, self.pos);
        match upper.as_str() {
            "AND" => Token::new(TokenKind::Amp, span),
            "OR" => Token::new(TokenKind::Pipe, span),
            "XOR" => Token::new(TokenKind::Xor, span),
            "NOT" => Token::new(TokenKind::Tilde, span),
            "MOD" => Token::new(TokenKind::Percent, span),
            _ => Token::new(TokenKind::Ident(word.to_owned()), span),
        }
    }

    fn lex_number(&mut self) -> Result<Token, CalcError> {
        let start = self.pos;
        // Radix prefixes.
        if self.src[start..].starts_with("0x")
            || self.src[start..].starts_with("0X")
            || self.src[start..].starts_with("0o")
            || self.src[start..].starts_with("0O")
            || self.src[start..].starts_with("0b")
            || self.src[start..].starts_with("0B")
        {
            let radix = match self.bytes[start + 1] {
                b'x' | b'X' => 16,
                b'o' | b'O' => 8,
                _ => 2,
            };
            self.pos += 2;
            let digits_start = self.pos;
            while self.pos < self.bytes.len() {
                let c = self.bytes[self.pos] as char;
                let ok = match radix {
                    16 => c.is_ascii_hexdigit() || c == '_',
                    8 => matches!(c, '0'..='7' | '_'),
                    _ => matches!(c, '0' | '1' | '_'),
                };
                if ok {
                    self.pos += 1;
                } else {
                    break;
                }
            }
            let raw = &self.src[start..self.pos];
            let digits = &self.src[digits_start..self.pos];
            if digits.is_empty() || digits == "_" {
                return Err(CalcError::invalid_token(
                    raw.to_owned(),
                    Span::new(start, self.pos),
                    "expected digits after radix prefix",
                ));
            }
            return match Number::from_int_str(raw, radix) {
                Ok(n) => Ok(Token::new(TokenKind::Number(n), Span::new(start, self.pos))),
                Err(_) => Err(CalcError::invalid_token(
                    raw.to_owned(),
                    Span::new(start, self.pos),
                    "malformed integer literal",
                )),
            };
        }

        // Decimal / float: digits [. digits] [(e|E) [+-] digits].
        let mut seen_digit = false;
        while self.pos < self.bytes.len()
            && (self.bytes[self.pos].is_ascii_digit() || self.bytes[self.pos] == b'_')
        {
            if self.bytes[self.pos].is_ascii_digit() {
                seen_digit = true;
            }
            self.pos += 1;
        }
        if self.peek() == Some(b'.') {
            // Only treat `.` as decimal point when followed by digit or when
            // we already saw digits (`"3."` is valid). A lone `"."` fails.
            let dot_followed_by_digit = matches!(self.peek2(), Some(c) if c.is_ascii_digit());
            if seen_digit || dot_followed_by_digit {
                self.pos += 1;
                while self.pos < self.bytes.len()
                    && (self.bytes[self.pos].is_ascii_digit() || self.bytes[self.pos] == b'_')
                {
                    if self.bytes[self.pos].is_ascii_digit() {
                        seen_digit = true;
                    }
                    self.pos += 1;
                }
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            let save = self.pos;
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            let exp_start = self.pos;
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
            if exp_start == self.pos {
                // Roll back: `"1e"` lexes as `1` then ident `e`.
                self.pos = save;
            } else {
                seen_digit = true;
            }
        }

        let raw = &self.src[start..self.pos];
        if !seen_digit {
            self.pos = start + 1;
            return Err(CalcError::invalid_token(
                raw.to_owned(),
                Span::new(start, self.pos),
                "malformed number",
            ));
        }
        match Number::from_decimal_str(raw) {
            Ok(n) => Ok(Token::new(TokenKind::Number(n), Span::new(start, self.pos))),
            Err(_) => Err(CalcError::invalid_token(
                raw.to_owned(),
                Span::new(start, self.pos),
                "malformed number",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        Lexer::new(src)
            .tokenize()
            .unwrap()
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    #[test]
    fn basic_arithmetic() {
        let k = kinds("1 + 2 * 3");
        assert_eq!(k.len(), 6); // 5 tokens + Eof
        assert!(matches!(k[0], TokenKind::Number(_)));
        assert!(matches!(k[1], TokenKind::Plus));
    }

    #[test]
    fn radix_literals() {
        let toks = Lexer::new("0xFF 0o17 0b1010").tokenize().unwrap();
        let vals: Vec<String> = toks
            .iter()
            .filter_map(|t| match &t.kind {
                TokenKind::Number(n) => Some(n.to_string()),
                _ => None,
            })
            .collect();
        assert_eq!(vals, vec!["255", "15", "10"]);
    }

    #[test]
    fn floats_and_scientific() {
        let toks = Lexer::new("3.14 1e-3 2.5E6").tokenize().unwrap();
        assert_eq!(toks.len(), 4);
        assert!(toks
            .iter()
            .all(|t| matches!(t.kind, TokenKind::Number(_) | TokenKind::Eof)));
    }

    #[test]
    fn keywords_fold_to_operators() {
        let k = kinds("1 AND 2 OR 3 XOR 4 NOT 5 MOD 6");
        assert!(k.contains(&TokenKind::Amp));
        assert!(k.contains(&TokenKind::Pipe));
        assert!(k.contains(&TokenKind::Xor));
        assert!(k.contains(&TokenKind::Tilde));
        assert!(k.contains(&TokenKind::Percent));
    }

    #[test]
    fn shifts_and_power() {
        let k = kinds("1 << 2 >> 3 ^ 4 ** 5");
        assert!(k.contains(&TokenKind::Shl));
        assert!(k.contains(&TokenKind::Shr));
        assert!(k.iter().filter(|t| **t == TokenKind::Caret).count() == 2);
    }

    #[test]
    fn invalid_char_errors_with_span() {
        let e = Lexer::new("@").tokenize().unwrap_err();
        assert!(matches!(e, CalcError::InvalidToken { .. }));
        assert!(e.render_with_source("@").contains('^'));
    }

    #[test]
    fn lone_dot_errors() {
        assert!(Lexer::new(".").tokenize().is_err());
    }

    #[test]
    fn spans_are_correct() {
        let toks = Lexer::new("12 + 34").tokenize().unwrap();
        assert_eq!(toks[0].span, Span::new(0, 2));
        assert_eq!(toks[1].span, Span::new(3, 4));
        assert_eq!(toks[2].span, Span::new(5, 7));
    }

    #[test]
    fn underscores_allowed() {
        let toks = Lexer::new("1_000_000 0xFF_FF").tokenize().unwrap();
        let vals: Vec<String> = toks
            .iter()
            .filter_map(|t| match &t.kind {
                TokenKind::Number(n) => Some(n.to_string()),
                _ => None,
            })
            .collect();
        assert_eq!(vals[0], "1000000");
        assert_eq!(vals[1], "65535");
    }
}
