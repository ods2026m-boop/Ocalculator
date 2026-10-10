//! Evaluator: AST → [`Number`] with variables, functions and modules.

use std::collections::HashMap;

use ocalc_functions::{lookup_constant, FunctionContext, FunctionRegistry};
use ocalc_modules::ModuleRegistry;
use ocalc_number::Number;

use crate::{AngleMode, BinOp, CalcError, Expr, ExprKind, UnOp};

/// Evaluation context: variables + functions + angle mode + plugins.
#[derive(Debug)]
pub struct Context {
    /// User variables (case-sensitive).
    pub variables: HashMap<String, Number>,
    /// Built-in functions.
    pub functions: FunctionRegistry,
    /// Third-party module providers.
    pub modules: ModuleRegistry,
    /// Trig angle mode.
    pub angle_mode: AngleMode,
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Context {
    /// Default context with all built-in functions.
    ///
    /// The merged [`FunctionRegistry`] is the angle-aware fast path; the same
    /// implementations are *also* registered as [`CalculatorModule`]s
    /// (`standard-math`, `scientific-math`, `statistics`, `units`) in
    /// [`modules`](Self::modules), so the plugin dispatch path serves them
    /// too. The evaluator never matches on concrete function names — it only
    /// talks to the two registries.
    #[must_use]
    pub fn new() -> Self {
        let mut ctx = Self {
            variables: HashMap::new(),
            functions: ocalc_functions::default_registry(),
            modules: ModuleRegistry::new(),
            angle_mode: AngleMode::Radians,
        };
        ctx.register_builtin_modules();
        ctx
    }

    /// Empty context (no built-in functions) — for sandboxing/tests.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            variables: HashMap::new(),
            functions: FunctionRegistry::new(),
            modules: ModuleRegistry::new(),
            angle_mode: AngleMode::Radians,
        }
    }

    /// Set the angle mode.
    pub fn set_angle_mode(&mut self, mode: AngleMode) {
        self.angle_mode = mode;
    }

    /// Define or overwrite a variable.
    pub fn set_var(&mut self, name: &str, value: Number) {
        self.variables.insert(name.to_owned(), value);
    }

    /// Define a variable, enforcing the host variable-count limit.
    ///
    /// # Errors
    /// [`crate::CalcError::LimitExceeded`] when inserting a *new* name would
    /// exceed `limits.max_variables`.
    pub fn set_var_limited(
        &mut self,
        name: &str,
        value: Number,
        limits: &crate::EngineLimits,
    ) -> Result<(), crate::CalcError> {
        if !self.variables.contains_key(name) && self.variables.len() >= limits.max_variables {
            return Err(crate::CalcError::limit(format!(
                "too many variables (limit {})",
                limits.max_variables
            )));
        }
        self.variables.insert(name.to_owned(), value);
        Ok(())
    }

    /// Borrow a variable.
    #[must_use]
    pub fn get_var(&self, name: &str) -> Option<&Number> {
        self.variables.get(name)
    }

    /// Sorted variable names (for UI inspection).
    #[must_use]
    pub fn variable_names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.variables.keys().cloned().collect();
        v.sort();
        v
    }

    /// Number of live variables.
    #[must_use]
    pub fn variable_count(&self) -> usize {
        self.variables.len()
    }

    /// Remove all variables (keeps functions/modules/angle mode).
    pub fn clear_vars(&mut self) {
        self.variables.clear();
    }

    /// Remove a variable. Returns `true` when it existed.
    pub fn remove_var(&mut self, name: &str) -> bool {
        self.variables.remove(name).is_some()
    }

    /// Register a plugin module (functions and/or units).
    ///
    /// # Errors
    /// [`ocalc_modules::ModuleError`] on duplicate ids or `init()` failures.
    pub fn register_module(
        &mut self,
        module: std::sync::Arc<dyn ocalc_modules::CalculatorModule>,
    ) -> Result<(), ocalc_modules::ModuleError> {
        self.modules.register(module)
    }

    /// Register the built-in modules (`standard-math`, `scientific-math`,
    /// `statistics`, `units`). Idempotent — duplicate ids are skipped.
    pub fn register_builtin_modules(&mut self) {
        use std::sync::Arc;
        let modules: Vec<Arc<dyn ocalc_modules::CalculatorModule>> = vec![
            Arc::new(ocalc_functions::StandardMathModule::new()),
            Arc::new(ocalc_functions::ScientificModule::new()),
            Arc::new(ocalc_functions::StatisticsModule::new()),
            Arc::new(ocalc_units::UnitsModule::new()),
        ];
        for m in modules {
            // Ignore duplicates so calling twice is harmless.
            let _ = self.modules.register(m);
        }
    }

    /// Function evaluation context derived from the angle mode.
    fn function_context(&self) -> FunctionContext {
        FunctionContext {
            angle_mode: self.angle_mode,
        }
    }
}

/// High-level calculator: owns a [`Context`] and evaluates source strings.
#[derive(Debug)]
pub struct Calculator {
    ctx: Context,
}

impl Default for Calculator {
    fn default() -> Self {
        Self::new()
    }
}

impl Calculator {
    /// New calculator with default context.
    #[must_use]
    pub fn new() -> Self {
        Self {
            ctx: Context::new(),
        }
    }

    /// New calculator in degree mode.
    #[must_use]
    pub fn with_degrees() -> Self {
        let mut c = Self::new();
        c.ctx.set_angle_mode(AngleMode::Degrees);
        c
    }

    /// Borrow the context mutably (to register modules, set vars, …).
    pub fn context_mut(&mut self) -> &mut Context {
        &mut self.ctx
    }

    /// Borrow the context.
    #[must_use]
    pub fn context(&self) -> &Context {
        &self.ctx
    }

    /// Parse, optimize (constant folding) and evaluate `src`.
    ///
    /// Optimization only folds `Number`-only arithmetic and safe identities;
    /// fallible folds are left for runtime so error spans still point at `src`.
    ///
    /// # Errors
    /// Any [`CalcError`] from lexing, parsing or evaluation.
    pub fn eval(&mut self, src: &str) -> Result<Number, CalcError> {
        let expr = crate::Parser::parse_source(src)?;
        let expr = crate::optimize(expr);
        self.eval_expr(&expr)
    }

    /// Evaluate an already-parsed expression.
    ///
    /// # Errors
    /// Evaluation [`CalcError`]s with spans from the AST.
    pub fn eval_expr(&mut self, expr: &Expr) -> Result<Number, CalcError> {
        eval(expr, &mut self.ctx)
    }
}

/// Evaluate `expr` in `ctx`.
///
/// Uses the default evaluation-depth guard
/// ([`crate::EngineLimits::default`].`max_eval_depth`).
///
/// # Errors
/// [`CalcError`] with AST spans attached.
pub fn eval(expr: &Expr, ctx: &mut Context) -> Result<Number, CalcError> {
    let max = crate::EngineLimits::default().max_eval_depth;
    eval_inner(expr, ctx, 0, max)
}

/// Evaluate with an explicit recursion guard (Phase 3 safe evaluation).
///
/// `max_depth` bounds AST nesting (parens, calls, unary chains *and* deep
/// left-associated arithmetic). Exceeding it yields
/// [`CalcError::LimitExceeded`] instead of a stack overflow.
///
/// # Errors
/// [`CalcError::LimitExceeded`] on excessive depth, else evaluation errors.
pub fn eval_limited(expr: &Expr, ctx: &mut Context, max_depth: usize) -> Result<Number, CalcError> {
    eval_inner(expr, ctx, 0, max_depth)
}

fn eval_inner(
    expr: &Expr,
    ctx: &mut Context,
    depth: usize,
    max_depth: usize,
) -> Result<Number, CalcError> {
    if depth > max_depth {
        return Err(CalcError::LimitExceeded {
            message: format!("evaluation depth exceeds limit {max_depth}"),
            span: Some(expr.span),
        });
    }
    let span = expr.span;
    match &expr.kind {
        ExprKind::Number(n) => Ok(n.clone()),
        ExprKind::Var(name) => {
            if let Some(v) = ctx.variables.get(name) {
                return Ok(v.clone());
            }
            // Greek aliases + case-insensitive constants.
            let alias = match name.as_str() {
                "π" => Some("pi"),
                "τ" => Some("tau"),
                "φ" => Some("phi"),
                _ => None,
            };
            if let Some(a) = alias {
                if let Some(c) = lookup_constant(a) {
                    return Ok(c);
                }
            }
            if let Some(c) = lookup_constant(name) {
                return Ok(c);
            }
            Err(CalcError::UnknownVariable {
                name: name.clone(),
                span: Some(span),
            })
        }
        ExprKind::Assign { name, expr: rhs } => {
            let v = eval_inner(rhs, ctx, depth + 1, max_depth)?;
            ctx.variables.insert(name.clone(), v.clone());
            Ok(v)
        }
        ExprKind::Unary { op, expr: inner } => {
            let v = eval_inner(inner, ctx, depth + 1, max_depth)?;
            match op {
                UnOp::Plus => Ok(v),
                UnOp::Neg => Ok(v.neg()),
                UnOp::BitNot => v.bitnot().map_err(|e| CalcError::from(e).with_span(span)),
            }
        }
        ExprKind::Factorial(inner) => {
            let v = eval_inner(inner, ctx, depth + 1, max_depth)?;
            v.factorial()
                .map_err(|e| CalcError::from(e).with_span(span))
        }
        ExprKind::Binary { op, left, right } => {
            let l = eval_inner(left, ctx, depth + 1, max_depth)?;
            let r = eval_inner(right, ctx, depth + 1, max_depth)?;
            apply_binary(*op, &l, &r).map_err(|e| e.with_span(span))
        }
        ExprKind::Call { name, args } => {
            let mut vals = Vec::with_capacity(args.len());
            for a in args {
                vals.push(eval_inner(a, ctx, depth + 1, max_depth)?);
            }
            call_function(name, &vals, ctx).map_err(|e| e.with_span(span))
        }
    }
}

/// Maximum AST depth (for diagnostics and pre-flight checks).
#[must_use]
pub fn expr_depth(expr: &Expr) -> usize {
    match &expr.kind {
        ExprKind::Number(_) | ExprKind::Var(_) => 1,
        ExprKind::Unary { expr: inner, .. } | ExprKind::Factorial(inner) => 1 + expr_depth(inner),
        ExprKind::Assign { expr: rhs, .. } => 1 + expr_depth(rhs),
        ExprKind::Binary { left, right, .. } => 1 + expr_depth(left).max(expr_depth(right)),
        ExprKind::Call { args, .. } => 1 + args.iter().map(expr_depth).max().unwrap_or(0),
    }
}

fn apply_binary(op: BinOp, l: &Number, r: &Number) -> Result<Number, CalcError> {
    match op {
        BinOp::Add => l.add(r).map_err(CalcError::from),
        BinOp::Sub => l.sub(r).map_err(CalcError::from),
        BinOp::Mul => l.mul(r).map_err(CalcError::from),
        BinOp::Div => l.div(r).map_err(CalcError::from),
        BinOp::Mod => l.rem(r).map_err(CalcError::from),
        BinOp::Pow => l.pow(r).map_err(CalcError::from),
        BinOp::BitAnd => l.bitand(r).map_err(CalcError::from),
        BinOp::BitOr => l.bitor(r).map_err(CalcError::from),
        BinOp::BitXor => l.bitxor(r).map_err(CalcError::from),
        BinOp::Shl => l.shl(r).map_err(CalcError::from),
        BinOp::Shr => l.shr(r).map_err(CalcError::from),
    }
}

fn call_function(name: &str, args: &[Number], ctx: &Context) -> Result<Number, CalcError> {
    // 1. Built-in registry (case-insensitive). Single hash lookup: `call`
    // already returns `Unknown` when absent, so no separate `contains` probe
    // (halves hashing + lowercase allocation on the hot path).
    match ctx.functions.call(name, args, &ctx.function_context()) {
        Ok(v) => return Ok(v),
        Err(ocalc_functions::FunctionError::Unknown(_)) => {} // fall to modules
        Err(e) => return Err(CalcError::from(e)),
    }
    // 2. Plugin modules — single dispatch scan (no prior `provides_function`
    // probe, which would scan + allocate a second time).
    match ctx.modules.call_function(name, args) {
        Ok(v) => Ok(v),
        Err(ocalc_modules::ModuleError::NotFound(_)) => Err(CalcError::UnknownFunction {
            name: name.to_owned(),
            span: None,
        }),
        Err(ocalc_modules::ModuleError::Provider(msg)) => Err(CalcError::MathError {
            message: msg,
            span: None,
        }),
        Err(other) => Err(CalcError::MathError {
            message: other.to_string(),
            span: None,
        }),
    }
}

/// Legacy helper kept for doctest compat: parse + eval with a fresh context.
#[allow(dead_code)]
fn _eval_fresh(src: &str) -> Result<Number, CalcError> {
    Calculator::new().eval(src)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(src: &str) -> Number {
        Calculator::new()
            .eval(src)
            .unwrap_or_else(|e| panic!("{src:?}: {e}"))
    }

    fn err(src: &str) -> CalcError {
        Calculator::new().eval(src).expect_err("expected error")
    }

    #[test]
    fn arithmetic_and_precedence() {
        assert_eq!(v("1 + 2 * 3"), Number::Integer(7));
        assert_eq!(v("(1 + 2) * 3"), Number::Integer(9));
        assert_eq!(v("10 - 4 - 3"), Number::Integer(3));
        assert_eq!(v("2^3^2").to_f64(), 512.0);
        assert_eq!(v("10 % 3"), Number::Integer(1));
    }

    #[test]
    fn example_from_spec() {
        // sin(45) in radians + sqrt(25)*10 — spec example shape.
        let r = v("sqrt(25) * 10");
        assert_eq!(r, Number::Float(50.0));
        let _ = v("sin(45) + sqrt(25) * 10"); // must not error
    }

    #[test]
    fn factorial_and_unary() {
        assert_eq!(v("5!"), Number::Integer(120));
        assert_eq!(v("3!!").to_string(), Number::Integer(720).to_string()); // (3!)! = 720
        assert_eq!(v("-3 + 5"), Number::Integer(2));
        assert_eq!(v("--5"), Number::Integer(5));
        assert_eq!(v("-2^2").to_f64(), -4.0); // unary binds looser than ^
    }

    #[test]
    fn variables_and_assignment() {
        let mut c = Calculator::new();
        assert_eq!(c.eval("x = 5").unwrap(), Number::Integer(5));
        assert_eq!(c.eval("x * 2").unwrap(), Number::Integer(10));
        assert_eq!(c.eval("x = x + 1").unwrap(), Number::Integer(6));
    }

    #[test]
    fn constants() {
        assert!((v("pi").to_f64() - std::f64::consts::PI).abs() < 1e-12);
        assert!((v("E").to_f64() - std::f64::consts::E).abs() < 1e-12);
        assert!((v("tau").to_f64() - std::f64::consts::TAU).abs() < 1e-12);
    }

    #[test]
    fn programmer_ops() {
        assert_eq!(v("0xFF"), Number::Integer(255));
        assert_eq!(v("0b1010"), Number::Integer(10));
        assert_eq!(v("0o17"), Number::Integer(15));
        assert_eq!(v("6 AND 3"), Number::Integer(2));
        assert_eq!(v("6 OR 3"), Number::Integer(7));
        assert_eq!(v("6 XOR 3"), Number::Integer(5));
        assert_eq!(v("1 << 8"), Number::Integer(256));
        assert_eq!(v("256 >> 8"), Number::Integer(1));
        assert_eq!(v("NOT 0"), Number::Integer(-1));
        assert_eq!(v("~0"), Number::Integer(-1));
    }

    #[test]
    fn division_by_zero() {
        assert!(matches!(err("1/0"), CalcError::DivisionByZero { .. }));
        assert!(matches!(err("1 % 0"), CalcError::DivisionByZero { .. }));
    }

    #[test]
    fn unknown_names() {
        assert!(matches!(
            err("nosuchfn(1)"),
            CalcError::UnknownFunction { .. }
        ));
        assert!(matches!(
            err("nosuchvar + 1"),
            CalcError::UnknownVariable { .. }
        ));
    }

    #[test]
    fn type_errors() {
        assert!(matches!(err("1.5 AND 2"), CalcError::TypeError { .. }));
    }

    #[test]
    fn degrees_mode() {
        let mut c = Calculator::with_degrees();
        let s = c.eval("sin(90)").unwrap();
        assert!((s.to_f64() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn spans_attached() {
        let e = err("1 + nosuchvar");
        assert!(e.render_with_source("1 + nosuchvar").contains('^'));
    }

    #[test]
    fn rational_exactness() {
        // 1/3 stays exact until forced to float.
        let r = v("1/3 + 1/3 + 1/3");
        assert!((r.to_f64() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn exact_rational_roundtrip_through_engine() {
        // The engine preserves `1/3` exactly: triple it and get integer 1.
        assert_eq!(v("1/3 + 1/3 + 1/3"), Number::Integer(1));
        assert_eq!(v("1/2 + 1/3").to_string(), "5/6");
        assert_eq!(v("2^-2").to_f64(), 0.25);
        assert_eq!(v("10!"), Number::Integer(3_628_800));
    }

    #[test]
    fn complex_flows_end_to_end() {
        // sqrt(-4) → 2i, and complex arithmetic composes.
        assert_eq!(v("sqrt(-4)").to_string(), "2i");
        assert_eq!(v("sqrt(-9)").to_string(), "3i");
        // ln(-1) → πi (real part ~0).
        let l = v("ln(-1)");
        assert!(l.to_string().contains('i'), "got {l}");
    }

    #[test]
    fn decimal_precision_in_engine() {
        // Decimal literals keep base-10 exactness through the engine.
        assert_eq!(v("0.1 + 0.2").to_string(), "0.3");
    }

    #[test]
    fn bigint_flows_end_to_end() {
        // 2^100 exceeds i64 — must promote, not wrap.
        let r = v("2^100");
        assert_eq!(r.to_string(), "1267650600228229401496703205376");
        assert!(matches!(r, Number::BigInt(_)));
        // Factorial beyond i64 stays exact.
        assert_eq!(v("25!").to_string(), "15511210043330985984000000");
    }

    #[test]
    fn variables_constants_and_calls_compose() {
        let mut c = Calculator::new();
        c.eval("x = 10").unwrap();
        assert_eq!(c.eval("x * 5").unwrap(), Number::Integer(50));
        assert_eq!(c.eval("sqrt(x)").unwrap().to_f64(), 10f64.sqrt());
        assert!((c.eval("pi").unwrap().to_f64() - std::f64::consts::PI).abs() < 1e-12);
        assert!((c.eval("phi").unwrap().to_f64() - std::f64::consts::GOLDEN_RATIO).abs() < 1e-12);
        assert_eq!(c.eval("mean(1, 2, 3, 4)").unwrap().to_f64(), 2.5);
        assert_eq!(c.eval("gcd(12, 18)").unwrap(), Number::Integer(6));
    }

    #[test]
    fn builtin_modules_are_registered() {
        let c = Calculator::new();
        let ids = c.context().modules.module_ids();
        for expected in ["standard-math", "scientific-math", "statistics", "units"] {
            assert!(
                ids.contains(&expected.to_owned()),
                "missing {expected}: {ids:?}"
            );
        }
        // Uniform inventory surface.
        let desc = c.context().modules.describe();
        assert_eq!(desc.len(), 4);
        // Capability gating works (sandbox example: pure-only hosts).
        let pure = c
            .context()
            .modules
            .with_capability(&ocalc_modules::Capability::Pure);
        assert_eq!(pure.len(), 4);
        let trig = c
            .context()
            .modules
            .with_capability(&ocalc_modules::Capability::Trigonometry);
        assert_eq!(trig, vec!["scientific-math".to_owned()]);
    }

    #[test]
    fn module_path_serves_functions_without_builtins() {
        // Empty functions registry + only the statistics module: the plugin
        // dispatch path (not the built-in registry) resolves `mean`.
        use std::sync::Arc;
        let mut ctx = Context::empty();
        ctx.modules
            .register(Arc::new(ocalc_functions::StatisticsModule::new()))
            .unwrap();
        let expr = crate::Parser::parse_source("mean(1, 2, 3)").unwrap();
        assert_eq!(eval(&expr, &mut ctx).unwrap().to_f64(), 2.0);
        // Unknown names still error.
        let expr = crate::Parser::parse_source("nosuchfn(1)").unwrap();
        assert!(matches!(
            eval(&expr, &mut ctx),
            Err(CalcError::UnknownFunction { .. })
        ));
    }

    #[test]
    fn custom_plugin_without_core_edits() {
        // Third-party module registered at runtime — the engine never names it.
        use ocalc_modules::{CalculatorModule, FunctionProvider, ModuleMetadata};
        use std::sync::Arc;
        struct Triple;
        impl FunctionProvider for Triple {
            fn function_names(&self) -> Vec<String> {
                vec!["triple".to_owned()]
            }
            fn call(
                &self,
                name: &str,
                args: &[ocalc_number::Number],
            ) -> Option<Result<ocalc_number::Number, String>> {
                if name == "triple" && args.len() == 1 {
                    Some(
                        args[0]
                            .mul(&ocalc_number::Number::Integer(3))
                            .map_err(|e| e.to_string()),
                    )
                } else {
                    None
                }
            }
        }
        struct TripleModule {
            provider: Triple,
        }
        impl CalculatorModule for TripleModule {
            fn metadata(&self) -> ModuleMetadata {
                ModuleMetadata::new("triple-test", "0.1.0", "test")
            }
            fn as_function_provider(&self) -> Option<&dyn FunctionProvider> {
                Some(&self.provider)
            }
        }
        let mut c = Calculator::new();
        c.context_mut()
            .register_module(Arc::new(TripleModule { provider: Triple }))
            .unwrap();
        assert_eq!(c.eval("triple(14)").unwrap(), Number::Integer(42));
        assert_eq!(c.eval("TRIPLE(1)").unwrap(), Number::Integer(3)); // case-insensitive
    }
}
