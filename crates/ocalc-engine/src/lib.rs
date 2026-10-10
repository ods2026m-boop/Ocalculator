//! `ocalc-engine`: core calculation engine.
//!
//! Pipeline:
//!
//! ```text
//! Input → Lexer → Tokens → Parser → AST → Evaluator → Result
//! ```
//!
//! ```rust
//! use ocalc_engine::Calculator;
//! let mut calc = Calculator::new();
//! // Precedence: sqrt(25) * 10 = 50
//! assert_eq!(calc.eval("sqrt(25) * 10").unwrap().to_f64(), 50.0);
//! // Full pipeline: lexer → parser → AST → evaluator
//! let v = calc.eval("sin(0) + sqrt(25) * 10 + 2^3").unwrap().to_f64();
//! assert!((v - 58.0).abs() < 1e-9);
//! ```
//!
//! ## Stable public API (Phase 3)
//!
//! Future frontends (Iced desktop app, CLI, ODS-os tools) must depend **only**
//! on these re-exports:
//!
//! - [`CalculatorEngine`] + [`EngineConfig`] + [`EngineLimits`]: guarded eval.
//! - [`Expression`]: parse-once, evaluate-many, versioned JSON.
//! - [`Value`]: exact numeric result (alias of `ocalc-number::Number`).
//! - [`ModuleRegistry`]: plugin discovery / validation / dispatch.
//! - [`FunctionRegistry`]: built-in math functions.
//! - [`CalcError`] + [`Span`]: spanned errors with caret rendering.
//! - [`GraphPoint`] + [`sample_expression`]: headless sampling for renderers.
//! - [`AngleMode`]: trig mode.
//!
//! The lower-level [`Calculator`], [`Context`], [`Lexer`], [`Parser`],
//! [`Expr`], [`BinOp`], [`UnOp`], [`Token`], [`optimize`] remain public for
//! advanced embedding and tests.

mod ast;
mod config;
mod engine;
mod error;
mod evaluator;
mod expression;
mod graph;
mod lexer;
mod optimize;
mod parser;
mod token;
mod value;

// ---- Stable API (frontends depend on these) ----
pub use config::{EngineConfig, EngineLimits};
pub use engine::CalculatorEngine;
pub use error::{CalcError, Span, SpanSuffix};
pub use evaluator::{eval, eval_limited, expr_depth, Calculator, Context};
pub use expression::{Expression, EXPRESSION_FORMAT_VERSION};
pub use graph::{points_to_json, sample_expression, sample_expression_limited, GraphPoint};
pub use ocalc_functions::{AngleMode, FunctionContext, FunctionRegistry};
pub use ocalc_modules::{
    CalculatorModule, Capability, FunctionProvider, ModuleMetadata, ModuleRegistry, ModuleSource,
    UnitProvider, ENGINE_PLUGIN_API_VERSION,
};
pub use value::{
    value_from_json_string, value_kind_name, value_to_json_string, Value, ValueExt,
    VALUE_FORMAT_VERSION,
};

// ---- Lower-level pipeline (advanced embedding) ----
pub use ast::{BinOp, Expr, ExprKind, UnOp};
pub use lexer::Lexer;
pub use optimize::{fold_constants, optimize};
pub use parser::Parser;
pub use token::{Token, TokenKind};
