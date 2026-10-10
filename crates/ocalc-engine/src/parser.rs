//! Parser: [`Token`] stream → [`Expr`] AST (recursive descent).
//!
//! Precedence (low → high):
//! `=` (right) → `OR`/`|` → `XOR` → `AND`/`&` → `<<` `>>` → `+` `-` →
//! `*` `/` `%` → unary `+` `-` `~`/`NOT` → `^`/`**` (right) → `!` → primary.

use crate::{CalcError, Expr, ExprKind, Span, Token, TokenKind};

/// Parser over an owned token vector.
#[derive(Debug)]
pub struct Parser {
    toks: Vec<Token>,
    pos: usize,
    limits: crate::EngineLimits,
    depth: usize,
}

impl Parser {
    /// Create a parser. `toks` must end with [`TokenKind::Eof`].
    #[must_use]
    pub fn new(toks: Vec<Token>) -> Self {
        Self {
            toks,
            pos: 0,
            limits: crate::EngineLimits::default(),
            depth: 0,
        }
    }

    /// Create a parser with explicit resource limits (Phase 3 hardening).
    #[must_use]
    pub fn with_limits(toks: Vec<Token>, limits: crate::EngineLimits) -> Self {
        Self {
            toks,
            pos: 0,
            limits,
            depth: 0,
        }
    }

    /// Parse tokens directly from source text.
    ///
    /// # Errors
    /// Lexer ([`CalcError::InvalidToken`]) or grammar ([`CalcError::SyntaxError`]) failures.
    pub fn parse_source(src: &str) -> Result<Expr, CalcError> {
        Self::parse_source_limited(src, &crate::EngineLimits::default())
    }

    /// Parse with explicit resource limits: source length, token count,
    /// recursion depth and argument count are all enforced.
    ///
    /// # Errors
    /// [`CalcError::LimitExceeded`] on resource violations, else lexer /
    /// grammar errors.
    pub fn parse_source_limited(
        src: &str,
        limits: &crate::EngineLimits,
    ) -> Result<Expr, CalcError> {
        limits.check_source_len(src)?;
        let toks = crate::Lexer::new(src).tokenize_limited(limits)?;
        limits.check_token_count(toks.len())?;
        Self::with_limits(toks, limits.clone()).parse()
    }

    /// Parse a full expression, requiring EOF afterwards.
    ///
    /// # Errors
    /// [`CalcError::SyntaxError`] on grammar violations or trailing tokens.
    pub fn parse(&mut self) -> Result<Expr, CalcError> {
        if self.peek_kind() == &TokenKind::Eof {
            return Err(CalcError::syntax(self.peek_span(), "empty expression"));
        }
        let e = self.parse_assign()?;
        match self.peek_kind() {
            TokenKind::Eof => Ok(e),
            _ => {
                let t = self.peek().clone();
                Err(CalcError::syntax(
                    t.span,
                    format!("unexpected trailing {}", t.kind.describe()),
                ))
            }
        }
    }

    // ---------------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------------

    fn peek(&self) -> &Token {
        &self.toks[self.pos.min(self.toks.len() - 1)]
    }

    fn peek2(&self) -> &Token {
        &self.toks[(self.pos + 1).min(self.toks.len() - 1)]
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn peek_span(&self) -> Span {
        self.peek().span
    }

    fn bump(&mut self) -> Token {
        let t = self.peek().clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn expect(
        &mut self,
        expected: &str,
        pred: impl Fn(&TokenKind) -> bool,
    ) -> Result<Token, CalcError> {
        let t = self.peek().clone();
        if pred(&t.kind) {
            Ok(self.bump())
        } else {
            Err(CalcError::syntax(
                t.span,
                format!("expected {expected}, found {}", t.kind.describe()),
            ))
        }
    }

    /// Enter one nesting level (parens / calls / assignment RHS / unary).
    fn enter(&mut self) -> Result<(), CalcError> {
        self.depth += 1;
        if self.depth > self.limits.max_recursion_depth {
            return Err(CalcError::LimitExceeded {
                message: format!(
                    "expression nesting exceeds limit {}",
                    self.limits.max_recursion_depth
                ),
                span: Some(self.peek_span()),
            });
        }
        Ok(())
    }

    fn exit(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    // ---------------------------------------------------------------
    // Grammar
    // ---------------------------------------------------------------

    fn parse_assign(&mut self) -> Result<Expr, CalcError> {
        self.enter()?;
        let out = self.parse_assign_inner();
        self.exit();
        out
    }

    fn parse_assign_inner(&mut self) -> Result<Expr, CalcError> {
        // `ident = rhs` — right associative. Match by reference to avoid
        // cloning the identifier (or number) on every precedence level.
        let is_assign_target = match self.peek_kind() {
            TokenKind::Ident(_) => matches!(self.peek2().kind, TokenKind::Assign),
            _ => false,
        };
        if is_assign_target {
            // Safe to clone now: we know this *is* an assignment.
            let name = match self.peek_kind() {
                TokenKind::Ident(n) => n.clone(),
                _ => unreachable!("checked above"),
            };
            let name_tok = self.bump();
            self.bump(); // `=`
            let rhs = self.parse_assign()?;
            let span = name_tok.span.merge(rhs.span);
            return Ok(Expr::new(
                ExprKind::Assign {
                    name,
                    expr: Box::new(rhs),
                },
                span,
            ));
        }
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, CalcError> {
        let mut left = self.parse_xor()?;
        while matches!(self.peek_kind(), TokenKind::Pipe) {
            self.bump();
            let right = self.parse_xor()?;
            let span = left.span.merge(right.span);
            left = Expr::new(
                ExprKind::Binary {
                    op: crate::BinOp::BitOr,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }
        Ok(left)
    }

    fn parse_xor(&mut self) -> Result<Expr, CalcError> {
        let mut left = self.parse_and()?;
        while matches!(self.peek_kind(), TokenKind::Xor) {
            self.bump();
            let right = self.parse_and()?;
            let span = left.span.merge(right.span);
            left = Expr::new(
                ExprKind::Binary {
                    op: crate::BinOp::BitXor,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, CalcError> {
        let mut left = self.parse_shift()?;
        while matches!(self.peek_kind(), TokenKind::Amp) {
            self.bump();
            let right = self.parse_shift()?;
            let span = left.span.merge(right.span);
            left = Expr::new(
                ExprKind::Binary {
                    op: crate::BinOp::BitAnd,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }
        Ok(left)
    }

    fn parse_shift(&mut self) -> Result<Expr, CalcError> {
        let mut left = self.parse_add()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Shl => crate::BinOp::Shl,
                TokenKind::Shr => crate::BinOp::Shr,
                _ => break,
            };
            self.bump();
            let right = self.parse_add()?;
            let span = left.span.merge(right.span);
            left = Expr::new(
                ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }
        Ok(left)
    }

    fn parse_add(&mut self) -> Result<Expr, CalcError> {
        let mut left = self.parse_mul()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Plus => crate::BinOp::Add,
                TokenKind::Minus => crate::BinOp::Sub,
                _ => break,
            };
            self.bump();
            let right = self.parse_mul()?;
            let span = left.span.merge(right.span);
            left = Expr::new(
                ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }
        Ok(left)
    }

    fn parse_mul(&mut self) -> Result<Expr, CalcError> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Star => crate::BinOp::Mul,
                TokenKind::Slash => crate::BinOp::Div,
                TokenKind::Percent => crate::BinOp::Mod,
                _ => break,
            };
            self.bump();
            let right = self.parse_unary()?;
            let span = left.span.merge(right.span);
            left = Expr::new(
                ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, CalcError> {
        match self.peek_kind() {
            TokenKind::Plus | TokenKind::Minus | TokenKind::Tilde => {
                self.enter()?;
                let t = self.bump();
                let op = match t.kind {
                    TokenKind::Plus => crate::UnOp::Plus,
                    TokenKind::Minus => crate::UnOp::Neg,
                    _ => crate::UnOp::BitNot,
                };
                let inner = self.parse_unary();
                self.exit();
                let inner = inner?;
                let span = t.span.merge(inner.span);
                Ok(Expr::new(
                    ExprKind::Unary {
                        op,
                        expr: Box::new(inner),
                    },
                    span,
                ))
            }
            _ => self.parse_power(),
        }
    }

    fn parse_power(&mut self) -> Result<Expr, CalcError> {
        let base = self.parse_postfix()?;
        if matches!(self.peek_kind(), TokenKind::Caret) {
            self.bump();
            // Right-assoc; exponent allows unary (`2^-3`, `-2^2` → -(2^2)
            // because unary is *below* power in the call chain... note the
            // exponent itself goes through parse_unary so signs bind there).
            let exp = self.parse_unary()?;
            let span = base.span.merge(exp.span);
            Ok(Expr::new(
                ExprKind::Binary {
                    op: crate::BinOp::Pow,
                    left: Box::new(base),
                    right: Box::new(exp),
                },
                span,
            ))
        } else {
            Ok(base)
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, CalcError> {
        let mut e = self.parse_primary()?;
        while matches!(self.peek_kind(), TokenKind::Bang) {
            let b = self.bump();
            let span = e.span.merge(b.span);
            e = Expr::new(ExprKind::Factorial(Box::new(e)), span);
        }
        Ok(e)
    }

    fn parse_primary(&mut self) -> Result<Expr, CalcError> {
        // Match by reference; clone the payload only on the taken branch so
        // hot numeric paths avoid an extra `Number` clone on every level.
        if matches!(self.peek_kind(), TokenKind::Number(_)) {
            let t = self.bump();
            let n = match t.kind {
                TokenKind::Number(v) => v,
                _ => unreachable!("checked above"),
            };
            return Ok(Expr::new(ExprKind::Number(n), t.span));
        }
        if matches!(self.peek_kind(), TokenKind::Ident(_)) {
            let t = self.bump();
            let name = match t.kind.clone() {
                TokenKind::Ident(n) => n,
                _ => unreachable!("checked above"),
            };
            // Reconstruct the token span (bump consumed it).
            let name_span = t.span;
            if matches!(self.peek_kind(), TokenKind::LParen) {
                self.enter()?;
                self.bump(); // `(`
                let mut args = Vec::new();
                if !matches!(self.peek_kind(), TokenKind::RParen) {
                    loop {
                        // Each argument goes through parse_assign (depth-checked).
                        let arg = self.parse_assign();
                        match arg {
                            Ok(a) => args.push(a),
                            Err(e) => {
                                self.exit();
                                return Err(e);
                            }
                        }
                        self.limits
                            .check_arg_count(args.len())
                            .map_err(|e| e.with_span(self.peek_span()))?;
                        if matches!(self.peek_kind(), TokenKind::Comma) {
                            self.bump();
                            if matches!(self.peek_kind(), TokenKind::RParen) {
                                self.exit();
                                return Err(CalcError::syntax(
                                    self.peek_span(),
                                    "trailing comma in argument list",
                                ));
                            }
                        } else {
                            break;
                        }
                    }
                }
                let close = self.expect("')'", |k| matches!(k, TokenKind::RParen));
                self.exit();
                let close = close?;
                let span = name_span.merge(close.span);
                return Ok(Expr::new(ExprKind::Call { name, args }, span));
            }
            return Ok(Expr::new(ExprKind::Var(name), name_span));
        }
        match self.peek_kind() {
            TokenKind::LParen => {
                self.enter()?;
                let open = self.bump();
                let inner = self.parse_assign();
                let inner = match inner {
                    Ok(v) => v,
                    Err(e) => {
                        self.exit();
                        return Err(e);
                    }
                };
                let close = self.expect("')'", |k| matches!(k, TokenKind::RParen));
                self.exit();
                let close = close?;
                let _ = open;
                // Keep inner span but extend to parens for error fidelity.
                let span = open.span.merge(close.span);
                Ok(Expr::new(inner.kind, span))
            }
            TokenKind::Minus | TokenKind::Plus | TokenKind::Tilde => {
                // Unreachable via parse_unary, but give a good error if we
                // somehow land here (e.g. after `(` with empty follow handled
                // below). Delegate upward.
                self.parse_unary()
            }
            other => {
                let t = self.peek().clone();
                if *other == TokenKind::Eof {
                    return Err(CalcError::syntax(t.span, "unexpected end of input"));
                }
                Err(CalcError::syntax(
                    t.span,
                    format!("unexpected {}", t.kind.describe()),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BinOp;

    fn parse(src: &str) -> Expr {
        Parser::parse_source(src).unwrap_or_else(|e| panic!("{src:?}: {e}"))
    }

    #[test]
    fn precedence_mul_over_add() {
        let e = parse("1 + 2 * 3");
        match e.kind {
            ExprKind::Binary { op: BinOp::Add, .. } => {}
            other => panic!("expected Add, got {other:?}"),
        }
    }

    #[test]
    fn power_right_associative() {
        // 2^3^2 = 2^(3^2)
        let e = parse("2^3^2");
        match e.kind {
            ExprKind::Binary {
                op: BinOp::Pow,
                ref right,
                ..
            } => match right.kind {
                ExprKind::Binary { op: BinOp::Pow, .. } => {}
                ref o => panic!("expected nested Pow on right, got {o:?}"),
            },
            other => panic!("expected Pow, got {other:?}"),
        }
    }

    #[test]
    fn factorial_binds_tightest() {
        let e = parse("3!^2");
        // (3!)^2
        match e.kind {
            ExprKind::Binary {
                op: BinOp::Pow,
                ref left,
                ..
            } => {
                assert!(matches!(left.kind, ExprKind::Factorial(_)));
            }
            other => panic!("expected Pow, got {other:?}"),
        }
    }

    #[test]
    fn assignment_right_associative() {
        let e = parse("x = y = 3");
        assert!(matches!(e.kind, ExprKind::Assign { .. }));
    }

    #[test]
    fn call_with_args() {
        let e = parse("max(1, 2, 3)");
        match e.kind {
            ExprKind::Call { name, args } => {
                assert_eq!(name, "max");
                assert_eq!(args.len(), 3);
            }
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn errors_on_empty_and_trailing() {
        assert!(Parser::parse_source("").is_err());
        assert!(Parser::parse_source("   ").is_err());
        assert!(Parser::parse_source("1 2").is_err());
        assert!(Parser::parse_source("(1").is_err());
        assert!(Parser::parse_source("1 +").is_err());
        assert!(Parser::parse_source("f(1,)").is_err());
    }

    #[test]
    fn shifts_and_bitwise_parse() {
        assert!(matches!(
            parse("1 << 2").kind,
            ExprKind::Binary { op: BinOp::Shl, .. }
        ));
        assert!(matches!(
            parse("1 AND 2").kind,
            ExprKind::Binary {
                op: BinOp::BitAnd,
                ..
            }
        ));
        assert!(matches!(
            parse("1 XOR 2").kind,
            ExprKind::Binary {
                op: BinOp::BitXor,
                ..
            }
        ));
    }

    #[test]
    fn edge_cases_have_spans_and_messages() {
        // Each failure must carry a span that renders carets against the source.
        for src in [
            "(1 + 2",  // unclosed paren
            "1 + * 3", // missing operand
            "* 3",     // leading operator
            "((1)",    // unbalanced
            "f(",      // unclosed call
            "f()",     // empty args are legal — checked separately
            "1 = 2",   // assignment target must be ident
            "x = = 3", // double assign op
            "2 ^^ 3",  // doubled power
            "1 2 3",   // juxtaposed literals
            "(,)",     // empty parens with comma
        ] {
            if src == "f()" {
                assert!(Parser::parse_source(src).is_ok(), "{src:?} should parse");
                continue;
            }
            let e = Parser::parse_source(src).expect_err(src);
            let rendered = e.render_with_source(src);
            assert!(rendered.contains('^'), "{src:?}: {rendered:?}");
        }
    }

    #[test]
    fn tricky_but_valid_forms() {
        // Chained unary, signed exponents, nested calls, deep parens.
        assert!(Parser::parse_source("---5").is_ok());
        assert!(Parser::parse_source("2^-3").is_ok());
        assert!(Parser::parse_source("2^(-3)").is_ok());
        assert!(Parser::parse_source("((((1))))").is_ok());
        assert!(Parser::parse_source("max(1, min(2, 3), sin(0))").is_ok());
        assert!(Parser::parse_source("x = 1 + 2 * 3").is_ok());
        assert!(Parser::parse_source("3!!").is_ok()); // (3!)!
        assert!(Parser::parse_source("NOT NOT 5").is_ok());
        // `**` is power.
        let e = parse("2 ** 3");
        assert!(matches!(e.kind, ExprKind::Binary { op: BinOp::Pow, .. }));
    }

    #[test]
    fn error_messages_guide_the_user() {
        let e = Parser::parse_source("(1 + 2").unwrap_err();
        assert!(e.to_string().contains("expected"), "got: {e}");
        let e = Parser::parse_source("1 +").unwrap_err();
        assert!(e.to_string().contains("unexpected end"), "got: {e}");
        let e = Parser::parse_source("1 2").unwrap_err();
        assert!(e.to_string().contains("trailing"), "got: {e}");
    }
}
