//! Abstract syntax tree.

use serde::{Deserialize, Serialize};

use crate::Span;

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `%` / `MOD`
    Mod,
    /// `^` / `**` (right-associative)
    Pow,
    /// `&` / `AND`
    BitAnd,
    /// `|` / `OR`
    BitOr,
    /// `XOR`
    BitXor,
    /// `<<`
    Shl,
    /// `>>`
    Shr,
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnOp {
    /// `+x`
    Plus,
    /// `-x`
    Neg,
    /// `~x` / `NOT x`
    BitNot,
}

/// An expression node with its source span.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Expr {
    /// Kind.
    pub kind: ExprKind,
    /// Source span (for errors + IDE highlighting).
    pub span: Span,
}

/// Expression kinds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExprKind {
    /// Numeric literal.
    Number(ocalc_number::Number),
    /// Variable reference.
    Var(String),
    /// Unary operation.
    Unary {
        /// Operator.
        op: UnOp,
        /// Operand.
        expr: Box<Expr>,
    },
    /// Binary operation.
    Binary {
        /// Operator.
        op: BinOp,
        /// Left side.
        left: Box<Expr>,
        /// Right side.
        right: Box<Expr>,
    },
    /// Postfix factorial: `expr!` (may chain: `3!!`).
    Factorial(Box<Expr>),
    /// Function call: `name(args…)`.
    Call {
        /// Function name as written.
        name: String,
        /// Arguments.
        args: Vec<Expr>,
    },
    /// Assignment: `name = expr` (right-associative).
    Assign {
        /// Target variable.
        name: String,
        /// Value expression.
        expr: Box<Expr>,
    },
}

impl Expr {
    /// Wrap a kind with a span.
    #[must_use]
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Self { kind, span }
    }
}
