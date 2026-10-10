//! Professional error system with source spans.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Byte-offset span into the source expression (half-open `[start, end)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Span {
    /// Start byte offset.
    pub start: usize,
    /// End byte offset (exclusive).
    pub end: usize,
}

impl Span {
    /// New span (normalises inverted ranges to empty at `start`).
    #[must_use]
    pub fn new(start: usize, end: usize) -> Self {
        if end < start {
            Self { start, end: start }
        } else {
            Self { start, end }
        }
    }

    /// Empty span at `pos`.
    #[must_use]
    pub fn point(pos: usize) -> Self {
        Self {
            start: pos,
            end: pos,
        }
    }

    /// Merge two spans (bounding box).
    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

/// All failures the engine can produce.
///
/// Phase 3 adds [`CalcError::LimitExceeded`] for resource-guard rejections
/// (expression too long, too many tokens, recursion too deep, too many
/// arguments, graph range too large). Frontends should present these like
/// syntax errors (user fixable) rather than internal bugs.
#[derive(Debug, Clone, PartialEq, Error, Serialize, Deserialize)]
pub enum CalcError {
    /// Unexpected character / malformed literal.
    #[error("invalid token {lexeme:?} at {span}: {message}")]
    InvalidToken {
        /// Offending text.
        lexeme: String,
        /// Location.
        span: Span,
        /// Reason.
        message: String,
    },
    /// Grammar violation.
    #[error("syntax error at {span}: {message}")]
    SyntaxError {
        /// Location.
        span: Span,
        /// What went wrong + expected.
        message: String,
    },
    /// Mathematically undefined (domain) failure.
    #[error("math error: {message}")]
    MathError {
        /// Reason.
        message: String,
        /// Location, when known.
        span: Option<Span>,
    },
    /// Integer / decimal overflow.
    #[error("overflow: {message}")]
    Overflow {
        /// Reason.
        message: String,
        /// Location, when known.
        span: Option<Span>,
    },
    /// Division (or modulo) by zero.
    #[error("division by zero{span_str}")]
    DivisionByZero {
        /// Location, when known.
        span: Option<Span>,
        /// Internal use: preformatted suffix.
        #[source]
        span_str: SpanSuffix,
    },
    /// Call to an unregistered function.
    #[error("unknown function: {name}")]
    UnknownFunction {
        /// Attempted name.
        name: String,
        /// Location, when known.
        span: Option<Span>,
    },
    /// Reference to an undefined variable.
    #[error("unknown variable: {name}")]
    UnknownVariable {
        /// Attempted name.
        name: String,
        /// Location, when known.
        span: Option<Span>,
    },
    /// Wrong argument count.
    #[error("function '{name}' arity error: expected {expected}, got {got}")]
    ArityError {
        /// Function name.
        name: String,
        /// Expected count / minimum.
        expected: usize,
        /// Supplied count.
        got: usize,
        /// Location, when known.
        span: Option<Span>,
    },
    /// Bitwise op on non-integers, factorial on non-integers, etc.
    #[error("type error: {message}")]
    TypeError {
        /// Reason.
        message: String,
        /// Location, when known.
        span: Option<Span>,
    },
    /// Resource guard rejected the input (too long, too deep, too many
    /// tokens/args/points). The message states the violated limit.
    #[error("limit exceeded: {message}")]
    LimitExceeded {
        /// Human-readable reason including the violated limit.
        message: String,
        /// Location, when known.
        span: Option<Span>,
    },
}

/// Helper so `DivisionByZero` can render `at <span>` only when spanned.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SpanSuffix(pub Option<Span>);

impl std::fmt::Display for SpanSuffix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(s) = self.0 {
            write!(f, " at {s}")
        } else {
            Ok(())
        }
    }
}

impl std::error::Error for SpanSuffix {}

impl CalcError {
    /// Attach a span to errors that support it (no-op otherwise).
    #[must_use]
    pub fn with_span(mut self, span: Span) -> Self {
        match &mut self {
            Self::MathError { span: s, .. }
            | Self::Overflow { span: s, .. }
            | Self::UnknownFunction { span: s, .. }
            | Self::UnknownVariable { span: s, .. }
            | Self::ArityError { span: s, .. }
            | Self::TypeError { span: s, .. }
            | Self::LimitExceeded { span: s, .. } => *s = Some(span),
            Self::DivisionByZero { span: s, span_str } => {
                *s = Some(span);
                span_str.0 = Some(span);
            }
            Self::InvalidToken { .. } | Self::SyntaxError { .. } => {}
        }
        self
    }

    /// Convenience: resource-limit rejection.
    #[must_use]
    pub fn limit(message: impl Into<String>) -> Self {
        Self::LimitExceeded {
            message: message.into(),
            span: None,
        }
    }

    /// True for resource-guard rejections.
    #[must_use]
    pub fn is_limit(&self) -> bool {
        matches!(self, Self::LimitExceeded { .. })
    }

    /// Convenience: syntax error at a span.
    #[must_use]
    pub fn syntax(span: Span, message: impl Into<String>) -> Self {
        Self::SyntaxError {
            span,
            message: message.into(),
        }
    }

    /// Convenience: invalid token.
    #[must_use]
    pub fn invalid_token(lexeme: String, span: Span, message: impl Into<String>) -> Self {
        Self::InvalidToken {
            lexeme,
            span,
            message: message.into(),
        }
    }

    /// Render with a source line: `message` + `\n  | <src>\n  | <carets>`.
    #[must_use]
    pub fn render_with_source(&self, src: &str) -> String {
        let span = match self {
            Self::InvalidToken { span, .. } | Self::SyntaxError { span, .. } => Some(*span),
            Self::MathError { span, .. }
            | Self::Overflow { span, .. }
            | Self::DivisionByZero { span, .. }
            | Self::UnknownFunction { span, .. }
            | Self::UnknownVariable { span, .. }
            | Self::ArityError { span, .. }
            | Self::TypeError { span, .. }
            | Self::LimitExceeded { span, .. } => *span,
        };
        let mut out = format!("{self}");
        if let Some(sp) = span {
            let start = sp.start.min(src.len());
            let end = sp.end.min(src.len()).max(start);
            let caret_len = (end - start).max(1);
            out.push_str(&format!(
                "\n  | {src}\n  | {}{}",
                " ".repeat(start),
                "^".repeat(caret_len)
            ));
        }
        out
    }
}

impl From<ocalc_number::NumberError> for CalcError {
    fn from(e: ocalc_number::NumberError) -> Self {
        use ocalc_number::NumberError as N;
        match e {
            N::DivisionByZero => Self::DivisionByZero {
                span: None,
                span_str: SpanSuffix(None),
            },
            N::Overflow(m) => Self::Overflow {
                message: m,
                span: None,
            },
            N::InvalidOperation(m) | N::Pow(m) => Self::MathError {
                message: m,
                span: None,
            },
            N::NonIntegerBitwise(m) => Self::TypeError {
                message: format!("bitwise operation requires integers, got {m}"),
                span: None,
            },
            N::FactorialDomain(m) => Self::MathError {
                message: format!("factorial undefined for {m}"),
                span: None,
            },
            N::NegativeShift(m) => Self::MathError {
                message: format!("negative shift: {m}"),
                span: None,
            },
            N::Parse(m) => Self::MathError {
                message: format!("invalid number: {m}"),
                span: None,
            },
        }
    }
}

impl From<ocalc_functions::FunctionError> for CalcError {
    fn from(e: ocalc_functions::FunctionError) -> Self {
        use ocalc_functions::FunctionError as F;
        match e {
            F::Arity {
                name,
                expected,
                got,
            } => Self::ArityError {
                name,
                expected,
                got,
                span: None,
            },
            F::Domain { name, message } => Self::MathError {
                message: format!("{name}: {message}"),
                span: None,
            },
            F::Number(n) => Self::from(n),
            F::Unknown(name) => Self::UnknownFunction { name, span: None },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_carets() {
        let e = CalcError::syntax(Span::new(2, 4), "expected ')'");
        let s = e.render_with_source("1 + * 3");
        assert!(s.contains("expected ')'"));
        assert!(s.contains('^'));
    }

    #[test]
    fn number_error_mapping() {
        use ocalc_number::NumberError as N;
        assert!(matches!(
            CalcError::from(N::DivisionByZero),
            CalcError::DivisionByZero { .. }
        ));
        assert!(matches!(
            CalcError::from(N::Overflow("x".into())),
            CalcError::Overflow { .. }
        ));
    }
}
