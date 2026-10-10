//! Error type for number operations.

use thiserror::Error;

/// Errors produced by [`crate::Number`] arithmetic.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NumberError {
    /// Division (or remainder) by zero.
    #[error("division by zero")]
    DivisionByZero,
    /// Integer overflow that could not be promoted.
    #[error("integer overflow: {0}")]
    Overflow(String),
    /// Operation is not defined for the given operand types.
    #[error("invalid operation: {0}")]
    InvalidOperation(String),
    /// Bitwise operations require integer operands.
    #[error("bitwise operation requires integer operands, got {0}")]
    NonIntegerBitwise(String),
    /// Factorial domain error.
    #[error("factorial undefined for {0}")]
    FactorialDomain(String),
    /// Negative shift amount.
    #[error("negative shift amount: {0}")]
    NegativeShift(String),
    /// Parse failure.
    #[error("invalid number literal: {0}")]
    Parse(String),
    /// Exponentiation domain / overflow.
    #[error("power error: {0}")]
    Pow(String),
}
