//! Function registry: name → implementation.
//!
//! The engine calls [`FunctionRegistry::call`] during evaluation. Custom
//! modules register additional [`MathFunction`]s without touching this crate
//! (see `ocalc-modules` for the plugin-side trait).

use std::collections::HashMap;
use std::sync::Arc;

use num_complex::Complex64;
use num_traits::{Signed, Zero};
use ocalc_number::Number;

use crate::trig::{expect_arity, expect_at_least, AngleMode};
use crate::{stats, FunctionError};

/// Evaluation context passed to every function (angle mode for trig).
#[derive(Debug, Clone, Copy)]
pub struct FunctionContext {
    /// How trig arguments/results are interpreted.
    pub angle_mode: AngleMode,
}

impl Default for FunctionContext {
    fn default() -> Self {
        Self {
            angle_mode: AngleMode::Radians,
        }
    }
}

/// Shared implementation type for math functions.
pub type MathFn =
    Arc<dyn Fn(&[Number], &FunctionContext) -> Result<Number, FunctionError> + Send + Sync>;

/// A callable mathematical function.
pub struct MathFunction {
    /// Canonical lowercase name.
    pub name: &'static str,
    /// Minimum arity (exact unless `variadic`).
    pub min_arity: usize,
    /// Exact arity when not variadic.
    pub max_arity: Option<usize>,
    /// Help text shown by `help(name)`.
    pub help: &'static str,
    /// Implementation.
    pub func: MathFn,
}

impl std::fmt::Debug for MathFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MathFunction")
            .field("name", &self.name)
            .field("min_arity", &self.min_arity)
            .field("max_arity", &self.max_arity)
            .field("help", &self.help)
            .finish_non_exhaustive()
    }
}

impl Clone for MathFunction {
    fn clone(&self) -> Self {
        Self {
            name: self.name,
            min_arity: self.min_arity,
            max_arity: self.max_arity,
            help: self.help,
            func: Arc::clone(&self.func),
        }
    }
}

impl MathFunction {
    /// Build a function from a plain closure.
    pub fn new(
        name: &'static str,
        min_arity: usize,
        max_arity: Option<usize>,
        help: &'static str,
        func: impl Fn(&[Number], &FunctionContext) -> Result<Number, FunctionError>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        Self {
            name,
            min_arity,
            max_arity,
            help,
            func: Arc::new(func),
        }
    }

    /// Check arity before dispatch.
    fn check_arity(&self, args: &[Number]) -> Result<(), FunctionError> {
        match self.max_arity {
            Some(max) if max == self.min_arity => expect_arity(self.name, args, max),
            _ => {
                expect_at_least(self.name, args, self.min_arity)?;
                if let Some(max) = self.max_arity {
                    if args.len() > max {
                        return Err(FunctionError::Arity {
                            name: self.name.to_owned(),
                            expected: max,
                            got: args.len(),
                        });
                    }
                }
                Ok(())
            }
        }
    }
}

/// Registry mapping lowercase names to functions.
#[derive(Debug, Default, Clone)]
pub struct FunctionRegistry {
    funcs: HashMap<String, MathFunction>,
}

impl FunctionRegistry {
    /// Empty registry (useful for sandboxing/tests).
    #[must_use]
    pub fn new() -> Self {
        Self {
            funcs: HashMap::new(),
        }
    }

    /// Register (or overwrite) a function. Names are lowercased.
    pub fn register(&mut self, f: MathFunction) {
        self.funcs.insert(f.name.to_ascii_lowercase(), f);
    }

    /// True if `name` (any case) is registered.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.funcs.contains_key(&name.to_ascii_lowercase())
    }

    /// Sorted list of registered names.
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.funcs.keys().cloned().collect();
        v.sort();
        v
    }

    /// Number of registered functions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.funcs.len()
    }

    /// True when no functions are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.funcs.is_empty()
    }

    /// Consume into the owned function list (for composing registries).
    #[must_use]
    pub fn into_functions(self) -> Vec<MathFunction> {
        self.funcs.into_values().collect()
    }

    /// Merge another registry into this one (other wins on conflicts).
    pub fn merge(&mut self, other: Self) {
        for f in other.into_functions() {
            self.register(f);
        }
    }

    /// Call a function by name (case-insensitive).
    pub fn call(
        &self,
        name: &str,
        args: &[Number],
        ctx: &FunctionContext,
    ) -> Result<Number, FunctionError> {
        let key = name.to_ascii_lowercase();
        let f = self
            .funcs
            .get(&key)
            .ok_or_else(|| FunctionError::Unknown(name.to_owned()))?;
        f.check_arity(args)?;
        (f.func)(args, ctx)
    }
}

// ----------------------------------------------------------------------
// Helpers operating on f64 with complex fallback
// ----------------------------------------------------------------------

fn unary_real(
    name: &str,
    args: &[Number],
    f: impl Fn(f64) -> f64,
) -> Result<Number, FunctionError> {
    expect_arity(name, args, 1)?;
    match &args[0] {
        Number::Complex(c) => {
            // Best-effort: apply to magnitude-preserving complex when the
            // closure is real-only. Callers needing true complex support
            // register dedicated closures (see sin/cos/... below).
            Ok(Number::Complex(Complex64::new(f(c.re), 0.0)))
        }
        other => Ok(Number::Float(f(other.to_f64()))),
    }
}

fn trig_direct(
    name: &str,
    args: &[Number],
    ctx: &FunctionContext,
    f: impl Fn(f64) -> f64,
    cf: impl Fn(Complex64) -> Complex64,
) -> Result<Number, FunctionError> {
    expect_arity(name, args, 1)?;
    match &args[0] {
        Number::Complex(c) => Ok(Number::from(cf(*c))),
        other => {
            let rad = ctx.angle_mode.to_radians(other.to_f64());
            Ok(Number::Float(f(rad)))
        }
    }
}

fn inv_trig(
    name: &str,
    args: &[Number],
    ctx: &FunctionContext,
    f: impl Fn(f64) -> f64,
    cf: impl Fn(Complex64) -> Complex64,
) -> Result<Number, FunctionError> {
    expect_arity(name, args, 1)?;
    match &args[0] {
        Number::Complex(c) => Ok(Number::from(cf(*c))),
        other => {
            let r = f(other.to_f64());
            Ok(Number::Float(ctx.angle_mode.from_radians(r)))
        }
    }
}

/// Build the default registry with all built-in functions.
///
/// Composed from [`standard_registry`] + [`scientific_registry`] +
/// [`statistics_registry`] so hosts can also mount each category as an
/// independent [`CalculatorModule`](ocalc_modules::CalculatorModule) (see
/// `crate::modules`). The engine itself never hardcodes names — it only calls
/// [`FunctionRegistry::call`].
#[must_use]
pub fn default_registry() -> FunctionRegistry {
    let mut r = FunctionRegistry::new();
    for f in standard_registry().into_functions() {
        r.register(f);
    }
    for f in scientific_registry().into_functions() {
        r.register(f);
    }
    for f in statistics_registry().into_functions() {
        r.register(f);
    }
    r
}

/// `standard-math`: rounding, min/max/clamp, integer helpers.
///
/// Pure, angle-independent — safe in every sandbox.
#[must_use]
pub fn standard_registry() -> FunctionRegistry {
    let mut r = FunctionRegistry::new();
    let mut reg =
        |name: &'static str, min: usize, max: Option<usize>, help: &'static str, f: MathFn| {
            r.register(MathFunction {
                name,
                min_arity: min,
                max_arity: max,
                help,
                func: f,
            });
        };

    // ---- Rounding / misc ----
    reg(
        "abs",
        1,
        Some(1),
        "absolute value (complex → magnitude)",
        Arc::new(|a, _| {
            expect_arity("abs", a, 1)?;
            Ok(a[0].abs())
        }),
    );
    reg(
        "floor",
        1,
        Some(1),
        "round down",
        Arc::new(|a, _| unary_real("floor", a, f64::floor)),
    );
    reg(
        "ceil",
        1,
        Some(1),
        "round up",
        Arc::new(|a, _| unary_real("ceil", a, f64::ceil)),
    );
    reg(
        "round",
        1,
        Some(1),
        "round to nearest",
        Arc::new(|a, _| unary_real("round", a, f64::round)),
    );
    reg(
        "trunc",
        1,
        Some(1),
        "truncate toward zero",
        Arc::new(|a, _| unary_real("trunc", a, f64::trunc)),
    );
    reg(
        "fract",
        1,
        Some(1),
        "fractional part",
        Arc::new(|a, _| unary_real("fract", a, f64::fract)),
    );
    reg(
        "sign",
        1,
        Some(1),
        "sign (-1, 0, 1)",
        Arc::new(|a, _| {
            expect_arity("sign", a, 1)?;
            let v = a[0].to_f64();
            Ok(Number::Float(if v > 0.0 {
                1.0
            } else if v < 0.0 {
                -1.0
            } else {
                0.0
            }))
        }),
    );
    reg(
        "min",
        1,
        None,
        "minimum of 1+ args",
        Arc::new(|a, _| {
            expect_at_least("min", a, 1)?;
            let mut best = a[0].to_f64();
            for v in &a[1..] {
                best = best.min(v.to_f64());
            }
            Ok(Number::Float(best))
        }),
    );
    reg(
        "max",
        1,
        None,
        "maximum of 1+ args",
        Arc::new(|a, _| {
            expect_at_least("max", a, 1)?;
            let mut best = a[0].to_f64();
            for v in &a[1..] {
                best = best.max(v.to_f64());
            }
            Ok(Number::Float(best))
        }),
    );
    reg(
        "clamp",
        3,
        Some(3),
        "clamp(x, lo, hi)",
        Arc::new(|a, _| {
            expect_arity("clamp", a, 3)?;
            Ok(Number::Float(
                a[0].to_f64().clamp(a[1].to_f64(), a[2].to_f64()),
            ))
        }),
    );

    // ---- Integer helpers ----
    reg(
        "fact",
        1,
        Some(1),
        "factorial (alias: factorial)",
        Arc::new(|a, _| {
            expect_arity("fact", a, 1)?;
            a[0].factorial().map_err(FunctionError::from)
        }),
    );
    reg(
        "factorial",
        1,
        Some(1),
        "factorial",
        Arc::new(|a, _| {
            expect_arity("factorial", a, 1)?;
            a[0].factorial().map_err(FunctionError::from)
        }),
    );
    reg(
        "gcd",
        2,
        Some(2),
        "greatest common divisor",
        Arc::new(|a, _| {
            expect_arity("gcd", a, 2)?;
            let x = a[0].as_bigint()?;
            let y = a[1].as_bigint()?;
            // Euclid on BigInt.
            let (mut m, mut n) = (x.abs(), y.abs());
            while !n.is_zero() {
                let r = m % &n;
                m = n;
                n = r;
            }
            Ok(Number::from(m))
        }),
    );
    reg(
        "lcm",
        2,
        Some(2),
        "least common multiple",
        Arc::new(|a, _| {
            expect_arity("lcm", a, 2)?;
            let x = a[0].as_bigint()?;
            let y = a[1].as_bigint()?;
            if x.is_zero() || y.is_zero() {
                return Ok(Number::Integer(0));
            }
            let g = {
                let (mut m, mut n) = (x.abs(), y.abs());
                while !n.is_zero() {
                    let r = m % &n;
                    m = n;
                    n = r;
                }
                m
            };
            Ok(Number::from((x.abs() / &g) * y.abs()))
        }),
    );

    r
}

/// `scientific-math`: trigonometry (angle-mode aware), logarithms,
/// exponentials and roots.
#[must_use]
pub fn scientific_registry() -> FunctionRegistry {
    let mut r = FunctionRegistry::new();
    let mut reg =
        |name: &'static str, min: usize, max: Option<usize>, help: &'static str, f: MathFn| {
            r.register(MathFunction {
                name,
                min_arity: min,
                max_arity: max,
                help,
                func: f,
            });
        };

    // ---- Trigonometry (angle-mode aware) ----
    reg(
        "sin",
        1,
        Some(1),
        "sine",
        Arc::new(|a, c| trig_direct("sin", a, c, f64::sin, |z| z.sin())),
    );
    reg(
        "cos",
        1,
        Some(1),
        "cosine",
        Arc::new(|a, c| trig_direct("cos", a, c, f64::cos, |z| z.cos())),
    );
    reg(
        "tan",
        1,
        Some(1),
        "tangent",
        Arc::new(|a, c| trig_direct("tan", a, c, f64::tan, |z| z.tan())),
    );
    reg(
        "asin",
        1,
        Some(1),
        "arcsine",
        Arc::new(|a, c| inv_trig("asin", a, c, f64::asin, |z| z.asin())),
    );
    reg(
        "acos",
        1,
        Some(1),
        "arccosine",
        Arc::new(|a, c| inv_trig("acos", a, c, f64::acos, |z| z.acos())),
    );
    reg(
        "atan",
        1,
        Some(1),
        "arctangent",
        Arc::new(|a, c| inv_trig("atan", a, c, f64::atan, |z| z.atan())),
    );
    reg(
        "sinh",
        1,
        Some(1),
        "hyperbolic sine",
        Arc::new(|a, _| unary_real("sinh", a, f64::sinh)),
    );
    reg(
        "cosh",
        1,
        Some(1),
        "hyperbolic cosine",
        Arc::new(|a, _| unary_real("cosh", a, f64::cosh)),
    );
    reg(
        "tanh",
        1,
        Some(1),
        "hyperbolic tangent",
        Arc::new(|a, _| unary_real("tanh", a, f64::tanh)),
    );
    reg(
        "atan2",
        2,
        Some(2),
        "two-argument arctangent atan2(y, x)",
        Arc::new(|a, ctx| {
            expect_arity("atan2", a, 2)?;
            let r = a[0].to_f64().atan2(a[1].to_f64());
            Ok(Number::Float(ctx.angle_mode.from_radians(r)))
        }),
    );

    // ---- Logarithms / exponentials / roots ----
    reg(
        "log",
        1,
        Some(2),
        "log(x) = log10(x); log(x, base) for arbitrary base",
        Arc::new(|a, _| match a.len() {
            1 => {
                let v = a[0].to_f64();
                if v <= 0.0 {
                    if v == 0.0 {
                        return Err(FunctionError::Domain {
                            name: "log".to_owned(),
                            message: "log(0) is undefined".to_owned(),
                        });
                    }
                    return Ok(Number::from(Complex64::new(v, 0.0).ln() / 10f64.ln()));
                }
                Ok(Number::Float(v.log10()))
            }
            2 => {
                let (v, b) = (a[0].to_f64(), a[1].to_f64());
                if v <= 0.0 || b <= 0.0 || b == 1.0 {
                    return Err(FunctionError::Domain {
                        name: "log".to_owned(),
                        message: "log(x, base) requires x > 0, base > 0, base != 1".to_owned(),
                    });
                }
                Ok(Number::Float(v.log(b)))
            }
            n => Err(FunctionError::Arity {
                name: "log".to_owned(),
                expected: 1,
                got: n,
            }),
        }),
    );
    reg(
        "ln",
        1,
        Some(1),
        "natural logarithm",
        Arc::new(|a, _| {
            expect_arity("ln", a, 1)?;
            a[0].ln().map_err(FunctionError::from)
        }),
    );
    reg(
        "log2",
        1,
        Some(1),
        "base-2 logarithm",
        Arc::new(|a, _| unary_real("log2", a, f64::log2)),
    );
    reg(
        "log10",
        1,
        Some(1),
        "base-10 logarithm",
        Arc::new(|a, _| unary_real("log10", a, f64::log10)),
    );
    reg(
        "exp",
        1,
        Some(1),
        "e^x",
        Arc::new(|a, _| {
            expect_arity("exp", a, 1)?;
            Ok(a[0].exp())
        }),
    );
    reg(
        "sqrt",
        1,
        Some(1),
        "square root (negative → complex)",
        Arc::new(|a, _| {
            expect_arity("sqrt", a, 1)?;
            Ok(a[0].sqrt())
        }),
    );
    reg(
        "cbrt",
        1,
        Some(1),
        "cube root (real, handles negatives)",
        Arc::new(|a, _| unary_real("cbrt", a, f64::cbrt)),
    );
    reg(
        "pow",
        2,
        Some(2),
        "power pow(base, exp)",
        Arc::new(|a, _| {
            expect_arity("pow", a, 2)?;
            a[0].pow(&a[1]).map_err(FunctionError::from)
        }),
    );
    reg(
        "hypot",
        2,
        Some(2),
        "hypotenuse hypot(x, y)",
        Arc::new(|a, _| {
            expect_arity("hypot", a, 2)?;
            Ok(Number::Float(a[0].to_f64().hypot(a[1].to_f64())))
        }),
    );

    r
}

/// `statistics`: variadic descriptive statistics.
#[must_use]
pub fn statistics_registry() -> FunctionRegistry {
    let mut r = FunctionRegistry::new();
    let mut reg =
        |name: &'static str, min: usize, max: Option<usize>, help: &'static str, f: MathFn| {
            r.register(MathFunction {
                name,
                min_arity: min,
                max_arity: max,
                help,
                func: f,
            });
        };

    // ---- Statistics (variadic, also accept previous via engine) ----
    reg(
        "sum",
        1,
        None,
        "sum of args",
        Arc::new(|a, _| {
            expect_at_least("sum", a, 1)?;
            stats::sum(a)
        }),
    );
    reg(
        "mean",
        1,
        None,
        "arithmetic mean",
        Arc::new(|a, _| {
            expect_at_least("mean", a, 1)?;
            stats::mean(a)
        }),
    );
    reg(
        "avg",
        1,
        None,
        "alias for mean",
        Arc::new(|a, _| {
            expect_at_least("avg", a, 1)?;
            stats::mean(a)
        }),
    );
    reg(
        "median",
        1,
        None,
        "median",
        Arc::new(|a, _| {
            expect_at_least("median", a, 1)?;
            stats::median(a)
        }),
    );
    reg(
        "variance",
        1,
        None,
        "population variance",
        Arc::new(|a, _| {
            expect_at_least("variance", a, 1)?;
            stats::variance(a)
        }),
    );
    reg(
        "stddev",
        1,
        None,
        "population std. deviation",
        Arc::new(|a, _| {
            expect_at_least("stddev", a, 1)?;
            stats::stddev(a)
        }),
    );

    r
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> FunctionContext {
        FunctionContext::default()
    }

    #[test]
    fn trig_radians_and_degrees() {
        let r = default_registry();
        let half_pi = Number::Float(std::f64::consts::FRAC_PI_2);
        let s = r.call("sin", &[half_pi], &ctx()).unwrap();
        assert!((s.to_f64() - 1.0).abs() < 1e-12);

        let deg = FunctionContext {
            angle_mode: AngleMode::Degrees,
        };
        let s2 = r.call("sin", &[Number::Float(90.0)], &deg).unwrap();
        assert!((s2.to_f64() - 1.0).abs() < 1e-12);
        let a = r.call("asin", &[Number::Float(1.0)], &deg).unwrap();
        assert!((a.to_f64() - 90.0).abs() < 1e-10);
    }

    #[test]
    fn logs_and_roots() {
        let r = default_registry();
        assert_eq!(
            r.call("sqrt", &[Number::Float(25.0)], &ctx())
                .unwrap()
                .to_f64(),
            5.0
        );
        assert_eq!(
            r.call("cbrt", &[Number::Float(-8.0)], &ctx())
                .unwrap()
                .to_f64(),
            -2.0
        );
        assert!(
            (r.call("ln", &[Number::Float(f64::exp(1.0))], &ctx())
                .unwrap()
                .to_f64()
                - 1.0)
                .abs()
                < 1e-12
        );
        assert_eq!(
            r.call("log", &[Number::Float(1000.0)], &ctx())
                .unwrap()
                .to_f64(),
            3.0
        );
    }

    #[test]
    fn unknown_and_arity() {
        let r = default_registry();
        assert!(matches!(
            r.call("nope", &[], &ctx()),
            Err(FunctionError::Unknown(_))
        ));
        assert!(matches!(
            r.call("sin", &[], &ctx()),
            Err(FunctionError::Arity { .. })
        ));
    }

    #[test]
    fn stats_and_misc() {
        let r = default_registry();
        let args = vec![Number::Float(1.0), Number::Float(2.0), Number::Float(3.0)];
        assert_eq!(r.call("sum", &args, &ctx()).unwrap().to_f64(), 6.0);
        assert_eq!(r.call("max", &args, &ctx()).unwrap().to_f64(), 3.0);
        assert_eq!(
            r.call("fact", &[Number::Integer(5)], &ctx()).unwrap(),
            Number::Integer(120)
        );
    }

    #[test]
    fn case_insensitive() {
        let r = default_registry();
        let v = r.call("SIN", &[Number::Float(0.0)], &ctx()).unwrap();
        assert_eq!(v.to_f64(), 0.0);
    }
}
