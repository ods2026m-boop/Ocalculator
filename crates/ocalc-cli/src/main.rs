//! `ocalc` — command-line frontend for the Ocalculator engine.
//!
//! Modes:
//! - `ocalc eval "<expr>"` — evaluate once and print.
//! - `ocalc` (no subcommand) — interactive REPL with line editing + persistent history.
//! - `ocalc graph …` — sample `y = f(x)` to CSV/JSON (UI renders later).
//! - `ocalc units …` — unit conversion.
//! - `ocalc functions [--search <q>]` — list functions (built-ins + scientific modules).
//! - `ocalc modules` — list discovered scientific modules.
//! - `ocalc history …` — inspect the persistent calculation history.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use clap::{Parser, Subcommand};
use ocalc_engine::{AngleMode, CalculatorEngine, CalculatorModule};
use ocalc_history::HistoryStore;
use rustyline::error::ReadlineError;

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

#[derive(Debug, Parser)]
#[command(
    name = "ocalc",
    version,
    about = "Modular calculator engine CLI (ODS-os)"
)]
struct Cli {
    /// Trig angle mode for this invocation.
    #[arg(long, value_enum, default_value = "radians", global = true)]
    angle: AngleArg,

    /// Path to the persistent JSON history file.
    #[arg(long, global = true)]
    history_file: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum AngleArg {
    Radians,
    Degrees,
}

impl AngleArg {
    fn mode(self) -> AngleMode {
        match self {
            Self::Radians => AngleMode::Radians,
            Self::Degrees => AngleMode::Degrees,
        }
    }
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Evaluate one expression and print the result.
    Eval {
        /// Expression, e.g. `sin(45) + sqrt(25) * 10`.
        expr: String,
        /// Print with debug formatting instead of display.
        #[arg(long)]
        debug: bool,
    },
    /// Start the interactive REPL (also the default with no subcommand).
    Repl,
    /// Sample `y = f(x)` into points (CSV by default).
    Graph {
        /// Expression in `x`, e.g. `sin(x)`.
        expr: String,
        /// Variable to sweep.
        #[arg(long, default_value = "x")]
        var: String,
        /// Range start.
        #[arg(long, default_value = "-10")]
        from: f64,
        /// Range end.
        #[arg(long, default_value = "10")]
        to: f64,
        /// Number of points (≥ 2).
        #[arg(long, default_value = "101")]
        points: usize,
        /// Output format: csv or json.
        #[arg(long, default_value = "csv")]
        format: String,
    },
    /// Convert a value between units: `ocalc units 1 km m`.
    Units {
        /// Numeric value.
        value: f64,
        /// Source unit symbol.
        from: String,
        /// Target unit symbol.
        to: String,
    },
    /// List functions (built-ins + scientific modules) and constants.
    Functions {
        /// Case-insensitive substring filter (empty = list all).
        #[arg(long)]
        search: Option<String>,
    },
    /// List discovered scientific modules (CAS, physics, chemistry, finance).
    Modules,
    /// Inspect persistent history.
    History {
        #[command(subcommand)]
        action: HistoryAction,
    },
}

#[derive(Debug, Subcommand)]
enum HistoryAction {
    /// Show the last N entries (newest first).
    Show {
        #[arg(long, default_value = "20")]
        limit: usize,
    },
    /// Case-insensitive substring search over expression + result.
    Search {
        /// Query string.
        query: String,
    },
    /// Clear all stored entries.
    Clear,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();
    let code = run(cli);
    std::process::exit(code);
}

fn run(cli: Cli) -> i32 {
    // Production boot: XDG dirs + legacy migration + startup log.
    // All best-effort — a read-only HOME must never break calculation.
    let _ = ocalc_history::ensure_dirs();
    let migrated = ocalc_history::migrate_legacy();
    ocalc_history::log_startup("ocalc", env!("CARGO_PKG_VERSION"));
    if migrated.settings_migrated || migrated.history_migrated {
        eprintln!(
            "info: migrated legacy data (settings: {}, history: {})",
            migrated.settings_migrated, migrated.history_migrated
        );
    }
    let history_path = cli
        .history_file
        .clone()
        .unwrap_or_else(default_history_path);
    match cli.command {
        Some(Command::Eval { expr, debug }) => {
            cmd_eval(&expr, cli.angle.mode(), &history_path, debug)
        }
        Some(Command::Repl) | None => {
            // No subcommand → REPL (classic calculator UX).
            if cli.command.is_none() && !atty() {
                // Piped stdin without subcommand: read lines non-interactively.
                return cmd_pipe(cli.angle.mode(), &history_path);
            }
            cmd_repl(cli.angle.mode(), &history_path)
        }
        Some(Command::Graph {
            expr,
            var,
            from,
            to,
            points,
            format,
        }) => cmd_graph(&expr, &var, from, to, points, &format, cli.angle.mode()),
        Some(Command::Units { value, from, to }) => cmd_units(value, &from, &to),
        Some(Command::Functions { search }) => cmd_functions(search.as_deref()),
        Some(Command::Modules) => cmd_modules(),
        Some(Command::History { action }) => cmd_history(action, &history_path),
    }
}

/// Scientific module discovery (Phase 6): independent crates registered
/// through the stable plugin API. The engine core is untouched.
fn discover_scientific_modules() -> Vec<Arc<dyn CalculatorModule>> {
    vec![
        Arc::new(ocalc_cas::CasModule::new()),
        Arc::new(ocalc_chemistry::ChemistryModule::new()),
        Arc::new(ocalc_finance::FinanceModule::new()),
        Arc::new(ocalc_physics::PhysicsModule::new()),
    ]
}

/// Factory for `:enable <id>` (single-module re-registration).
fn create_module_by_id(id: &str) -> Option<Arc<dyn CalculatorModule>> {
    match id {
        "cas" => Some(Arc::new(ocalc_cas::CasModule::new())),
        "chemistry" => Some(Arc::new(ocalc_chemistry::ChemistryModule::new())),
        "finance" => Some(Arc::new(ocalc_finance::FinanceModule::new())),
        "physics" => Some(Arc::new(ocalc_physics::PhysicsModule::new())),
        _ => None,
    }
}

/// Case-insensitive substring search over function names.
fn filter_functions(names: Vec<String>, query: Option<&str>) -> Vec<String> {
    match query {
        None => names,
        Some(q) => {
            let q = q.to_ascii_lowercase();
            if q.trim().is_empty() {
                return names;
            }
            names
                .into_iter()
                .filter(|n| n.to_ascii_lowercase().contains(&q))
                .collect()
        }
    }
}

/// True when stdin is a TTY (interactive). Without `libc` we shell out to
/// `test -t 0`; failure (e.g. Windows) conservatively assumes interactive.
fn atty() -> bool {
    std::process::Command::new("test")
        .arg("-t")
        .arg("0")
        .status()
        .map(|s| s.success())
        .unwrap_or(true)
}

// ---------------------------------------------------------------------------
// Subcommands
// ---------------------------------------------------------------------------

fn new_calculator(mode: AngleMode) -> CalculatorEngine {
    let mut c = CalculatorEngine::new();
    c.set_angle_mode(mode);
    // Scientific module discovery through the stable engine API
    // (`register_module_checked` + `ENGINE_PLUGIN_API_VERSION`), shared with
    // the desktop frontend (`ocalc-desktop/src/modules.rs`). Duplicates are
    // skipped (idempotent); real failures are reported loudly (never silent)
    // via stderr + production log.
    for m in discover_scientific_modules() {
        let id = m.name();
        match c.register_module_checked(m, ocalc_engine::ENGINE_PLUGIN_API_VERSION) {
            Ok(()) => {}
            Err(ocalc_modules::ModuleError::AlreadyRegistered(_)) => {}
            Err(e) => {
                let msg = format!("module '{id}' failed to load: {e}");
                eprintln!("warning: {msg}");
                ocalc_history::log_event("module", &msg);
            }
        }
    }
    c
}

fn load_history(path: &std::path::Path) -> HistoryStore {
    if path.exists() {
        match HistoryStore::load_from_file(path) {
            Ok(h) => h,
            Err(e) => {
                let msg = format!("could not load history {}: {e}", path.display());
                eprintln!("warning: {msg}");
                ocalc_history::log_event("history", &msg);
                HistoryStore::new()
            }
        }
    } else {
        HistoryStore::new()
    }
}

fn save_history(store: &HistoryStore, path: &std::path::Path) {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(parent);
        }
    }
    if let Err(e) = store.save_to_file(path) {
        let msg = format!("could not save history {}: {e}", path.display());
        eprintln!("warning: {msg}");
        ocalc_history::log_event("history", &msg);
    }
}

fn cmd_eval(expr: &str, mode: AngleMode, history_path: &std::path::Path, debug: bool) -> i32 {
    let mut calc = new_calculator(mode);
    let t = Instant::now();
    match calc.eval(expr) {
        Ok(v) => {
            if debug {
                println!("{v:?}");
            } else {
                println!("{v}");
            }
            let mut h = load_history(history_path);
            h.push(expr, &v.to_string());
            // Attach duration to the in-memory entry is skipped for the
            // one-shot path; persist directly.
            let _ = t.elapsed();
            save_history(&h, history_path);
            0
        }
        Err(e) => {
            eprintln!("error: {}", e.render_with_source(expr));
            1
        }
    }
}

fn cmd_pipe(mode: AngleMode, history_path: &std::path::Path) -> i32 {
    use std::io::BufRead;
    let mut calc = new_calculator(mode);
    let mut store = load_history(history_path);
    let mut failures = 0;
    for line in std::io::stdin().lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match calc.eval(line) {
            Ok(v) => {
                println!("{v}");
                store.push(line, &v.to_string());
            }
            Err(e) => {
                eprintln!("error: {}", e.render_with_source(line));
                failures += 1;
            }
        }
    }
    save_history(&store, history_path);
    if failures > 0 {
        1
    } else {
        0
    }
}

fn cmd_graph(
    expr: &str,
    var: &str,
    from: f64,
    to: f64,
    n: usize,
    format: &str,
    mode: AngleMode,
) -> i32 {
    // Same stable engine API as the desktop graph tab
    // (`CalculatorEngine::sample_graph`); the CLI only renders CSV/JSON.
    let calc = new_calculator(mode);
    match calc.sample_graph(expr, var, from, to, n) {
        Ok(pts) => match format.to_ascii_lowercase().as_str() {
            "csv" => {
                println!("x,y,valid");
                for p in &pts {
                    let y = if p.y.is_nan() {
                        String::new()
                    } else {
                        p.y.to_string()
                    };
                    println!("{},{},{}", p.x, y, p.valid);
                }
                0
            }
            "json" => {
                match serde_json::to_string_pretty(
                    &pts.iter()
                        .map(|p| serde_json::json!({"x": p.x, "y": p.y, "valid": p.valid}))
                        .collect::<Vec<_>>(),
                ) {
                    Ok(j) => {
                        println!("{j}");
                        0
                    }
                    Err(e) => {
                        eprintln!("error: cannot serialise points: {e}");
                        1
                    }
                }
            }
            other => {
                eprintln!("error: unknown --format {other:?}, expected csv|json");
                1
            }
        },
        Err(e) => {
            eprintln!("error: {}", e.render_with_source(expr));
            1
        }
    }
}

fn cmd_units(value: f64, from: &str, to: &str) -> i32 {
    let conv = ocalc_units::builtin_converter();
    match conv.convert(value, from, to) {
        Ok(v) => {
            println!("{v}");
            0
        }
        Err(e) => {
            eprintln!("error: {e}");
            eprintln!("hint: `ocalc units --help` takes VALUE FROM TO, e.g. `ocalc units 1 km m`");
            1
        }
    }
}

fn cmd_functions(search: Option<&str>) -> i32 {
    // Merged inventory through the stable engine API (same call the desktop
    // function finder uses): built-ins + registered scientific modules.
    let calc = new_calculator(AngleMode::Radians);
    let names = filter_functions(calc.function_names(), search);
    if let Some(q) = search {
        println!("Functions matching {q:?} ({}):", names.len());
    } else {
        println!("Functions ({}):", names.len());
    }
    for name in &names {
        println!("  {name}");
    }
    if search.is_none() {
        println!("\nConstants:");
        for c in ocalc_functions::CONSTANTS {
            println!("  {:<10} = {}  ({})", c.name, c.value, c.description);
        }
        println!("\nOperators: + - * / % ^(**) ! &|XOR ~ << >> ( ) , =");
        println!("Keywords (case-insensitive): AND OR XOR NOT MOD");
        println!("Literals: 42 3.14 1e-3 0xFF 0o17 0b1010");
    }
    0
}

fn cmd_modules() -> i32 {
    let calc = new_calculator(AngleMode::Radians);
    println!(
        "Scientific modules ({}):",
        discover_scientific_modules().len()
    );
    // Same stable inventory API the desktop settings panel uses.
    for (name, version, funcs, _caps) in calc.describe_modules() {
        // Only show the scientific four (builtins stay in `functions`).
        if ["cas", "chemistry", "finance", "physics"].contains(&name.as_str()) {
            println!("  {name} v{version}: {} function(s)", funcs.len());
            let mut f = funcs;
            f.sort();
            for name in f {
                println!("    - {name}");
            }
        }
    }
    println!("\nNative symbolic APIs (not engine-callable):");
    println!("  cas: simplify(x+x)=2*x, differentiate(x^2)=2*x, solve, factor, expand");
    println!("  chemistry: parse_formula(\"H2O\"), molecular_mass(\"CO2\")");
    println!("  physics: velocity/distance/time helpers use the unit system");
    println!("  finance: percent/compound/loan_payment/growth helpers");
    0
}

fn cmd_history(action: HistoryAction, path: &std::path::Path) -> i32 {
    let mut store = load_history(path);
    match action {
        HistoryAction::Show { limit } => {
            for e in store.last(limit) {
                println!(
                    "#{} [{}] {} = {}",
                    e.id,
                    e.timestamp.format("%Y-%m-%d %H:%M:%S"),
                    e.expression,
                    e.result
                );
            }
            0
        }
        HistoryAction::Search { query } => {
            let hits = store.search(&query);
            if hits.is_empty() {
                println!("no matches for {query:?}");
            } else {
                for e in hits {
                    println!("#{} {} = {}", e.id, e.expression, e.result);
                }
            }
            0
        }
        HistoryAction::Clear => {
            store.clear();
            save_history(&store, path);
            println!("history cleared");
            0
        }
    }
}

// ---------------------------------------------------------------------------
// REPL
// ---------------------------------------------------------------------------

const REPL_HELP: &str = "\
Commands (prefix with ':'):
  :help              show this help
  :quit | :exit      leave the REPL
  :history [N]       show last N entries (this session + persisted)
  :search <query>    search history
  :clear             clear screen (and :clear history wipes stored history)
  :vars              list variables
  :forget <name>     delete a variable
  :degrees           trig in degrees
  :radians           trig in radians
  :functions [query] list functions (optional substring filter)
  :modules           list scientific modules (cas/physics/chemistry/finance)
  :enable <id>       enable a scientific module
  :disable <id>      disable a scientific module
  :units             list some common units
  :graph <expr> <from> <to> [points]   sample y=f(x), prints CSV head
Everything else is evaluated as a math expression.
";

fn cmd_repl(mode: AngleMode, history_path: &std::path::Path) -> i32 {
    let mut calc = new_calculator(mode);
    let mut store = load_history(history_path);
    // Seed readline history with past expressions for ↑ recall.
    let mut rl = match rustyline::DefaultEditor::new() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: cannot start line editor: {e}");
            return 1;
        }
    };
    for e in store.last(500).iter().rev() {
        let _ = rl.add_history_entry(e.expression.as_str());
    }

    println!(
        "Ocalculator {} — type :help for commands, :quit to exit.",
        env!("CARGO_PKG_VERSION")
    );
    println!(
        "angle mode: {}",
        match calc.angle_mode() {
            AngleMode::Radians => "radians",
            AngleMode::Degrees => "degrees",
        }
    );

    loop {
        let line = rl.readline("ocalc> ");
        match line {
            Ok(raw) => {
                let text = raw.trim().to_owned();
                if text.is_empty() {
                    continue;
                }
                let _ = rl.add_history_entry(text.as_str());
                if text.starts_with(':') {
                    if repl_command(&text, &mut calc, &mut store, history_path) {
                        break;
                    }
                    continue;
                }
                let t = Instant::now();
                match calc.eval(&text) {
                    Ok(v) => {
                        println!("{v}");
                        let us = t.elapsed().as_micros().min(u64::MAX as u128) as u64;
                        let entry = store.push(&text, &v.to_string());
                        let _ = (entry, us);
                        save_history(&store, history_path);
                    }
                    Err(e) => {
                        eprintln!("error: {}", e.render_with_source(&text));
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("(Ctrl-C — type :quit to exit)");
            }
            Err(ReadlineError::Eof) => {
                println!("bye");
                break;
            }
            Err(e) => {
                eprintln!("error: readline: {e}");
                return 1;
            }
        }
    }
    0
}

/// Handle a `:` command. Returns true when the REPL should exit.
fn repl_command(
    text: &str,
    calc: &mut CalculatorEngine,
    store: &mut HistoryStore,
    history_path: &std::path::Path,
) -> bool {
    let mut parts = text[1..].split_whitespace();
    let cmd = parts.next().unwrap_or("").to_ascii_lowercase();
    let rest: Vec<&str> = parts.collect();
    match cmd.as_str() {
        "quit" | "exit" | "q" => {
            println!("bye");
            true
        }
        "help" | "h" | "?" => {
            print!("{REPL_HELP}");
            false
        }
        "history" => {
            let n: usize = rest.first().and_then(|s| s.parse().ok()).unwrap_or(20);
            for e in store.last(n) {
                println!("#{} {} = {}", e.id, e.expression, e.result);
            }
            false
        }
        "search" => {
            let q = rest.join(" ");
            for e in store.search(&q) {
                println!("#{} {} = {}", e.id, e.expression, e.result);
            }
            false
        }
        "clear" => {
            if rest.first().is_some_and(|s| *s == "history") {
                store.clear();
                save_history(store, history_path);
                println!("history cleared");
            } else {
                print!("\x1b[2J\x1b[H");
            }
            false
        }
        "vars" | "variables" => {
            let names = calc.variable_names();
            if names.is_empty() {
                println!("(no variables — assign with `x = 5`)");
            } else {
                for n in names {
                    if let Some(v) = calc.get_var(&n) {
                        println!("{n} = {v}");
                    }
                }
            }
            false
        }
        "forget" => {
            match rest.first() {
                Some(name) if calc.remove_var(name) => println!("forgot {name}"),
                Some(name) => println!("no such variable: {name}"),
                None => println!("usage: :forget <name>"),
            }
            false
        }
        "degrees" | "deg" => {
            calc.set_angle_mode(AngleMode::Degrees);
            println!("angle mode: degrees");
            false
        }
        "radians" | "rad" => {
            calc.set_angle_mode(AngleMode::Radians);
            println!("angle mode: radians");
            false
        }
        "functions" | "funcs" => {
            let q = rest.first().copied();
            let _ = cmd_functions(q);
            false
        }
        "modules" => {
            let _ = cmd_modules();
            false
        }
        "enable" => {
            match rest.first() {
                Some(id) => match create_module_by_id(id) {
                    Some(m) => {
                        match calc
                            .register_module_checked(m, ocalc_engine::ENGINE_PLUGIN_API_VERSION)
                        {
                            Ok(()) => println!("module {id} enabled"),
                            Err(ocalc_modules::ModuleError::AlreadyRegistered(_)) => {
                                println!("module {id} is already enabled")
                            }
                            Err(e) => {
                                eprintln!("cannot enable {id}: {e}");
                                ocalc_history::log_event(
                                    "module",
                                    &format!("cannot enable {id}: {e}"),
                                );
                            }
                        }
                    }
                    None => eprintln!("unknown module {id:?} (try :modules)"),
                },
                None => println!("usage: :enable <module-id>"),
            }
            false
        }
        "disable" => {
            match rest.first() {
                Some(id) => {
                    if calc.unregister_module(id) {
                        println!("module {id} disabled");
                    } else {
                        eprintln!("module {id:?} not registered (try :modules)");
                    }
                }
                None => println!("usage: :disable <module-id>"),
            }
            false
        }
        "units" => {
            let c = ocalc_units::builtin_converter();
            let mut syms = c.symbols();
            syms.sort();
            println!("{}", syms.join(" "));
            false
        }
        "graph" => {
            // :graph <expr> <from> <to> [points]
            if rest.len() < 3 {
                println!("usage: :graph <expr> <from> <to> [points]");
                return false;
            }
            let (expr, from, to) = (rest[0], rest[1].parse::<f64>(), rest[2].parse::<f64>());
            let n: usize = rest.get(3).and_then(|s| s.parse().ok()).unwrap_or(11);
            match (from, to) {
                (Ok(a), Ok(b)) => match calc.sample_graph(expr, "x", a, b, n) {
                    Ok(pts) => {
                        println!("x,y,valid");
                        for p in pts.iter().take(20) {
                            println!("{},{},{}", p.x, p.y, p.valid);
                        }
                        if pts.len() > 20 {
                            println!("… ({} points total)", pts.len());
                        }
                    }
                    Err(e) => eprintln!("error: {e}"),
                },
                _ => println!("usage: :graph <expr> <from> <to> [points]"),
            }
            false
        }
        other => {
            eprintln!("unknown command :{other} — type :help");
            false
        }
    }
}

/// Default history file: `<XDG data>/ocalculator/history.json`
/// (e.g. `~/.local/share/ocalculator/history.json`).
/// Legacy `~/.local/share/ocalc/history.json` is migrated on startup.
fn default_history_path() -> PathBuf {
    ocalc_history::history_path()
}
