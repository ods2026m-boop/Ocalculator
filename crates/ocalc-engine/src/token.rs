//! Token definitions shared by the lexer and parser.

use ocalc_number::Number;

use crate::Span;

/// Lexical token kinds.
///
/// Keywords `AND`/`OR`/`XOR`/`NOT`/`MOD` are folded into operator kinds at
/// lex time (case-insensitive) so the parser sees a uniform stream.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// Numeric literal (already parsed).
    Number(Number),
    /// Identifier or keyword-treated-as-ident (function / variable names).
    Ident(String),
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `%` or keyword `MOD`
    Percent,
    /// `^` or `**` (power)
    Caret,
    /// `!` (postfix factorial)
    Bang,
    /// `&` or keyword `AND`
    Amp,
    /// `|` or keyword `OR`
    Pipe,
    /// Keyword `XOR` (no symbolic form — `^` is power)
    Xor,
    /// `~` or keyword `NOT` (unary bitwise not)
    Tilde,
    /// `<<`
    Shl,
    /// `>>`
    Shr,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `,`
    Comma,
    /// `=`
    Assign,
    /// End of input.
    Eof,
}

impl TokenKind {
    /// Human-readable name used in syntax errors.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Number(n) => format!("number {n}"),
            Self::Ident(s) => format!("identifier '{s}'"),
            Self::Plus => "'+'".to_owned(),
            Self::Minus => "'-'".to_owned(),
            Self::Star => "'*'".to_owned(),
            Self::Slash => "'/'".to_owned(),
            Self::Percent => "'%'".to_owned(),
            Self::Caret => "'^'".to_owned(),
            Self::Bang => "'!'".to_owned(),
            Self::Amp => "'&'".to_owned(),
            Self::Pipe => "'|'".to_owned(),
            Self::Xor => "'XOR'".to_owned(),
            Self::Tilde => "'~'".to_owned(),
            Self::Shl => "'<<'".to_owned(),
            Self::Shr => "'>>'".to_owned(),
            Self::LParen => "'('".to_owned(),
            Self::RParen => "')'".to_owned(),
            Self::Comma => "','".to_owned(),
            Self::Assign => "'='".to_owned(),
            Self::Eof => "end of input".to_owned(),
        }
    }
}

/// A token with its source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// Kind.
    pub kind: TokenKind,
    /// Source span.
    pub span: Span,
}

impl Token {
    /// New token.
    #[must_use]
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}
