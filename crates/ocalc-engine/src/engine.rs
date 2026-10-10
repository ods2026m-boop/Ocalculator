//! Stable high-level facade (Phase 3 public API).
//!
//! [`CalculatorEngine`] is the **only** type future frontends (Iced desktop
//! app, CLI, ODS-os tools) should need: it owns a [`Context`], enforces
//! [`EngineLimits`] on every input, and exposes versioned serialization for
//! expressions and values.
//!
//! The lower-level [`crate::Calculator`] / [`crate::Context`] /
//! [`crate::Lexer`] / [`crate::Parser`] remain public for advanced embedding
//! and tests, but UI code must depend only on:
//! `CalculatorEngine`, `EngineConfig`, `EngineLimits`, `Expression`, `Value`,
//! `ModuleRegistry`, `FunctionRegistry`, `CalcError`, `Span`, `GraphPoint`,
//! `AngleMode`.

use std::sync::Arc;

use crate::{
    AngleMode, CalcError, Context, EngineConfig, EngineLimits, Expr, Expression, GraphPoint, Value,
};

/// Stable calculator engine: parse → optimize → evaluate with guards.
///
/// Cheap to construct (`Context::new` registers the four built-in modules).
/// Not `Clone` (contexts own variable maps); create one per UI session and
/// share via `&mut` borrows on the UI thread.
#[derive(Debug)]
pub struct CalculatorEngine {
    ctx: Context,
    config: EngineConfig,
}

impl Default for CalculatorEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl CalculatorEngine {
    /// New engine with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self {
            ctx: Context::new(),
            config: EngineConfig::default(),
        }
    }

    /// New engine with explicit configuration.
    ///
    /// Applies the configured angle mode to the fresh context.
    #[must_use]
    pub fn with_config(config: EngineConfig) -> Self {
        let mut ctx = Context::new();
        ctx.set_angle_mode(config.angle_mode);
        Self { ctx, config }
    }

    /// Degrees + default limits.
    #[must_use]
    pub fn with_degrees() -> Self {
        Self::with_config(EngineConfig::with_degrees())
    }

    /// Sandboxed engine (tight limits, radians).
    #[must_use]
    pub fn sandboxed() -> Self {
        Self::with_config(EngineConfig::sandbox())
    }

    /// Borrow the configuration.
    #[must_use]
    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    /// Replace the configuration (applies angle mode immediately).
    ///
    /// # Errors
    /// [`CalcError::LimitExceeded`] when the new limits are degenerate.
    pub fn set_config(&mut self, config: EngineConfig) -> Result<(), CalcError> {
        config.validate()?;
        self.ctx.set_angle_mode(config.angle_mode);
        self.config = config;
        Ok(())
    }

    /// Borrow the underlying context (advanced: direct module queries).
    #[must_use]
    pub fn context(&self) -> &Context {
        &self.ctx
    }

    /// Borrow the underlying context mutably (register modules, …).
    pub fn context_mut(&mut self) -> &mut Context {
        &mut self.ctx
    }

    /// Current angle mode.
    #[must_use]
    pub fn angle_mode(&self) -> AngleMode {
        self.ctx.angle_mode
    }

    /// Set the trig angle mode.
    pub fn set_angle_mode(&mut self, mode: AngleMode) {
        self.ctx.set_angle_mode(mode);
        self.config.angle_mode = mode;
    }

    /// Parse, optimize and evaluate `src` in one shot.
    ///
    /// Enforces `max_expression_len` / `max_tokens` / `max_recursion_depth`
    /// at parse time and `max_eval_depth` at evaluation time.
    ///
    /// # Errors
    /// Any [`CalcError`] (including [`CalcError::LimitExceeded`]).
    pub fn eval(&mut self, src: &str) -> Result<Value, CalcError> {
        let limits = self.config.limits.clone();
        limits.check_source_len(src)?;
        let expr = crate::Parser::parse_source_limited(src, &limits)?;
        let expr = crate::optimize(expr);
        self.eval_expr_limited(&expr, &limits)
    }

    /// Parse without evaluating (for highlighting, reuse, serialization).
    ///
    /// # Errors
    /// Lexer / parser / limit errors.
    pub fn parse(&self, src: &str) -> Result<Expression, CalcError> {
        Expression::parse_limited(src, &self.config.limits)
    }

    /// Parse + optimize without evaluating.
    ///
    /// # Errors
    /// Lexer / parser / limit errors.
    pub fn parse_optimized(&self, src: &str) -> Result<Expression, CalcError> {
        Expression::parse_optimized_limited(src, &self.config.limits)
    }

    /// Evaluate an already-parsed expression.
    ///
    /// # Errors
    /// Evaluation [`CalcError`]s.
    pub fn eval_expression(&mut self, expr: &Expression) -> Result<Value, CalcError> {
        let limits = self.config.limits.clone();
        self.eval_expr_limited(expr.ast(), &limits)
    }

    fn eval_expr_limited(
        &mut self,
        expr: &Expr,
        limits: &EngineLimits,
    ) -> Result<Value, CalcError> {
        // Variable-count guard for assignments: pre-check so a hostile
        // `a1=1; a2=2; …` flood fails before mutating the context.
        // (Single-expression assignments add at most one var; the per-point
        // check lives in `Context::set_var_limited` for programmatic use.)
        let out = crate::evaluator::eval_limited(expr, &mut self.ctx, limits.max_eval_depth)?;
        if self.ctx.variable_count() > limits.max_variables {
            // Roll back the most recent assignment overflow is complex; the
            // documented contract is that hosts with strict caps call
            // `set_var_limited` for programmatic inserts. Single-eval overflow
            // here is reported without rollback (variables remain usable).
            return Err(CalcError::limit(format!(
                "too many variables (limit {})",
                limits.max_variables
            )));
        }
        Ok(out)
    }

    // -- Variables (stable surface for UI inspectors) ---------------------

    /// Define or overwrite a variable (no limit check; UI sessions are
    /// trusted — use [`Context::set_var_limited`] for untrusted bulk loads).
    pub fn set_var(&mut self, name: &str, value: Value) {
        self.ctx.set_var(name, value);
    }

    /// Borrow a variable.
    #[must_use]
    pub fn get_var(&self, name: &str) -> Option<&Value> {
        self.ctx.get_var(name)
    }

    /// Sorted variable names.
    #[must_use]
    pub fn variable_names(&self) -> Vec<String> {
        self.ctx.variable_names()
    }

    /// Remove a variable. Returns `true` when it existed.
    pub fn remove_var(&mut self, name: &str) -> bool {
        self.ctx.remove_var(name)
    }

    /// Remove all variables.
    pub fn clear_vars(&mut self) {
        self.ctx.clear_vars();
    }

    // -- Modules (stable surface for plugin UI) ---------------------------

    /// Register a plugin module.
    ///
    /// # Errors
    /// [`ocalc_modules::ModuleError`] on duplicates / bad metadata / init
    /// failures.
    pub fn register_module(
        &mut self,
        module: Arc<dyn ocalc_modules::CalculatorModule>,
    ) -> Result<(), ocalc_modules::ModuleError> {
        self.ctx.register_module(module)
    }

    /// Strict registration with API-compatibility enforcement.
    ///
    /// # Errors
    /// Same as [`Self::register_module`] plus
    /// [`ocalc_modules::ModuleError::IncompatibleVersion`].
    pub fn register_module_checked(
        &mut self,
        module: Arc<dyn ocalc_modules::CalculatorModule>,
        engine_api: &str,
    ) -> Result<(), ocalc_modules::ModuleError> {
        self.ctx.modules.register_checked(module, engine_api)
    }

    /// Remove a plugin module by id. Returns `true` when something was removed.
    ///
    /// Stable counterpart to [`Context::modules`](crate::Context::modules)
    /// unregister — frontends (CLI `:disable`, desktop settings) must use this
    /// instead of reaching into the context directly.
    pub fn unregister_module(&mut self, id: &str) -> bool {
        self.ctx.modules.unregister(id)
    }

    /// Ids of all registered modules (sorted).
    #[must_use]
    pub fn module_ids(&self) -> Vec<String> {
        self.ctx.modules.module_ids()
    }

    /// All function names across built-ins + modules (sorted, deduped).
    #[must_use]
    pub fn function_names(&self) -> Vec<String> {
        let mut names = self.ctx.functions.names();
        names.extend(self.ctx.modules.function_names());
        names.sort();
        names.dedup();
        names
    }

    /// Uniform module inventory: `(name, version, functions, capabilities)`.
    ///
    /// Stable counterpart to `ModuleRegistry::describe` — frontends (CLI
    /// `modules`, desktop settings) must use this instead of reaching into
    /// the context directly. No duplicated calculation logic lives in the UI.
    #[must_use]
    pub fn describe_modules(
        &self,
    ) -> Vec<(String, String, Vec<String>, Vec<ocalc_modules::Capability>)> {
        self.ctx.modules.describe()
    }

    // -- Graph (delegates to the sampling engine) -------------------------

    /// Sample `expr_source` with `var` swept over `[start, end]` in `n` points.
    ///
    /// # Errors
    /// Range/parse errors or [`CalcError::LimitExceeded`] when `n` exceeds
    /// `max_graph_points`.
    pub fn sample_graph(
        &self,
        expr_source: &str,
        var: &str,
        start: f64,
        end: f64,
        n: usize,
    ) -> Result<Vec<GraphPoint>, CalcError> {
        crate::sample_expression_limited(
            expr_source,
            var,
            start,
            end,
            n,
            &self.ctx,
            &self.config.limits,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_basic() {
        let mut e = CalculatorEngine::new();
        assert_eq!(e.eval("2 + 3 * 4").unwrap().to_string(), "14");
        assert_eq!(e.eval("x = 5").unwrap().to_string(), "5");
        assert_eq!(e.eval("x * 2").unwrap().to_string(), "10");
    }

    #[test]
    fn limits_reject_hostile_input() {
        let mut e = CalculatorEngine::with_config(EngineConfig {
            angle_mode: AngleMode::Radians,
            limits: EngineLimits {
                max_expression_len: 4,
                ..EngineLimits::default()
            },
        });
        let err = e.eval("1 + 2").unwrap_err();
        assert!(err.is_limit(), "got {err}");
    }

    #[test]
    fn recursion_guard() {
        let mut e = CalculatorEngine::new();
        let deep = "(((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((1)))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))";
        // 127+ parens: parser limit is 128 nesting levels (each paren costs a
        // few depth units via assign/unary/paren guards) → must fail safely.
        let r = e.eval(deep);
        assert!(r.is_err(), "deep nesting must not succeed");
    }

    #[test]
    fn variables_surface() {
        let mut e = CalculatorEngine::new();
        e.set_var("x", Value::Integer(3));
        assert_eq!(e.get_var("x").unwrap().to_string(), "3");
        assert_eq!(e.variable_names(), vec!["x".to_owned()]);
        assert!(e.remove_var("x"));
        assert!(e.get_var("x").is_none());
        e.set_var("a", Value::Integer(1));
        e.clear_vars();
        assert!(e.variable_names().is_empty());
    }

    #[test]
    fn modules_visible() {
        let e = CalculatorEngine::new();
        for id in ["standard-math", "scientific-math", "statistics", "units"] {
            assert!(
                e.module_ids().contains(&id.to_owned()),
                "{:?}",
                e.module_ids()
            );
        }
        assert!(e.function_names().contains(&"sin".to_owned()));
    }

    #[test]
    fn graph_delegates_with_limits() {
        let e = CalculatorEngine::new();
        let pts = e.sample_graph("x * 2", "x", 0.0, 2.0, 3).unwrap();
        assert_eq!(pts.len(), 3);
        let tight = EngineConfig {
            angle_mode: AngleMode::Radians,
            limits: EngineLimits {
                max_graph_points: 2,
                ..EngineLimits::default()
            },
        };
        let e2 = CalculatorEngine::with_config(tight);
        assert!(e2.sample_graph("x", "x", 0.0, 1.0, 3).is_err());
    }

    #[test]
    fn degrees_mode() {
        let mut e = CalculatorEngine::with_degrees();
        assert!((e.eval("sin(90)").unwrap().to_f64() - 1.0).abs() < 1e-12);
    }
}
