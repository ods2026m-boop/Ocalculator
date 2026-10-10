//! XDG base directories, Ocalculator file locations, legacy migration and
//! production file logging.
//!
//! Layout for v1.0.0 (XDG compliant, no hardcoded `~/.config/ocalc-desktop`):
//!
//! ```text
//! Config: $XDG_CONFIG_HOME/ocalculator/  (fallback ~/.config/ocalculator/)
//!   └── settings.json          desktop settings (versioned, see ocalc-desktop)
//! Data:   $XDG_DATA_HOME/ocalculator/    (fallback ~/.local/share/ocalculator/)
//!   ├── history.json           CLI persistent calculation history
//!   └── logs/ocalculator.log   production log (startup / errors / modules / UI)
//! Cache:  $XDG_CACHE_HOME/ocalculator/   (fallback ~/.cache/ocalculator/)
//! ```
//!
//! Migration is best-effort and non-destructive: when the new location does
//! not exist yet but a legacy file does, the content is copied over and the
//! event is logged. Legacy files are kept as backups.
//!
//! Legacy locations:
//! - `~/.config/ocalc-desktop/settings.json` → new `settings.json`
//! - `~/.local/share/ocalc/history.json` → new `history.json`
//!
//! All filesystem operations are best-effort (`ensure_dirs`, `log_event`,
//! `migrate_legacy` never panic and never fail the caller); hard failures
//! are reported by returning `io::Result` only from `ensure_dirs`.

use std::path::{Path, PathBuf};

/// Application directory name under each XDG base dir.
pub const APP_DIR_NAME: &str = "ocalculator";
/// Desktop settings file name inside the config dir.
pub const SETTINGS_FILE_NAME: &str = "settings.json";
/// CLI history file name inside the data dir.
pub const HISTORY_FILE_NAME: &str = "history.json";
/// Production log file name inside the log dir.
pub const LOG_FILE_NAME: &str = "ocalculator.log";

// ---------------------------------------------------------------------------
// Pure directory resolution (testable without touching the environment)
// ---------------------------------------------------------------------------

/// Resolve the config dir from explicit inputs (pure, for tests and hosts
/// that already resolved the environment).
///
/// Priority: `xdg_config_home` (when non-empty) → `home/.config` →
/// relative `./.ocalculator` fallback (no `HOME` set, e.g. minimal
/// containers — keeps debug builds working).
#[must_use]
pub fn config_dir_for(home: Option<&str>, xdg_config_home: Option<&str>) -> PathBuf {
    if let Some(xdg) = xdg_config_home.map(str::trim).filter(|s| !s.is_empty()) {
        return PathBuf::from(xdg).join(APP_DIR_NAME);
    }
    match home.map(str::trim).filter(|s| !s.is_empty()) {
        Some(h) => PathBuf::from(h).join(".config").join(APP_DIR_NAME),
        None => PathBuf::from(".").join(APP_DIR_NAME),
    }
}

/// Resolve the data dir from explicit inputs (pure).
#[must_use]
pub fn data_dir_for(home: Option<&str>, xdg_data_home: Option<&str>) -> PathBuf {
    if let Some(xdg) = xdg_data_home.map(str::trim).filter(|s| !s.is_empty()) {
        return PathBuf::from(xdg).join(APP_DIR_NAME);
    }
    match home.map(str::trim).filter(|s| !s.is_empty()) {
        Some(h) => PathBuf::from(h).join(".local/share").join(APP_DIR_NAME),
        None => PathBuf::from(".").join(APP_DIR_NAME),
    }
}

/// Resolve the cache dir from explicit inputs (pure).
#[must_use]
pub fn cache_dir_for(home: Option<&str>, xdg_cache_home: Option<&str>) -> PathBuf {
    if let Some(xdg) = xdg_cache_home.map(str::trim).filter(|s| !s.is_empty()) {
        return PathBuf::from(xdg).join(APP_DIR_NAME);
    }
    match home.map(str::trim).filter(|s| !s.is_empty()) {
        Some(h) => PathBuf::from(h).join(".cache").join(APP_DIR_NAME),
        None => PathBuf::from(".").join(APP_DIR_NAME),
    }
}

// ---------------------------------------------------------------------------
// Environment-backed resolution (production entry points)
// ---------------------------------------------------------------------------

fn env_opt(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

/// XDG config dir for this process (`$XDG_CONFIG_HOME` first).
#[must_use]
pub fn config_dir() -> PathBuf {
    config_dir_for(
        env_opt("HOME").as_deref(),
        env_opt("XDG_CONFIG_HOME").as_deref(),
    )
}

/// XDG data dir for this process (`$XDG_DATA_HOME` first).
#[must_use]
pub fn data_dir() -> PathBuf {
    data_dir_for(
        env_opt("HOME").as_deref(),
        env_opt("XDG_DATA_HOME").as_deref(),
    )
}

/// XDG cache dir for this process (`$XDG_CACHE_HOME` first).
#[must_use]
pub fn cache_dir() -> PathBuf {
    cache_dir_for(
        env_opt("HOME").as_deref(),
        env_opt("XDG_CACHE_HOME").as_deref(),
    )
}

/// Production log directory: `<data>/logs`.
#[must_use]
pub fn log_dir() -> PathBuf {
    data_dir().join("logs")
}

/// Default desktop settings file: `<config>/settings.json`.
#[must_use]
pub fn settings_path() -> PathBuf {
    config_dir().join(SETTINGS_FILE_NAME)
}

/// Default CLI history file: `<data>/history.json`.
#[must_use]
pub fn history_path() -> PathBuf {
    data_dir().join(HISTORY_FILE_NAME)
}

/// Production log file: `<data>/logs/ocalculator.log`.
#[must_use]
pub fn log_file_path() -> PathBuf {
    log_dir().join(LOG_FILE_NAME)
}

/// Legacy (pre-1.0) desktop settings file. Kept for migration only.
#[must_use]
pub fn legacy_settings_path() -> PathBuf {
    match env_opt("HOME") {
        Some(h) => PathBuf::from(h).join(".config/ocalc-desktop/settings.json"),
        None => PathBuf::from(".ocalc-desktop-settings.json"),
    }
}

/// Legacy (pre-1.0) CLI history file. Kept for migration only.
#[must_use]
pub fn legacy_history_path() -> PathBuf {
    match env_opt("HOME") {
        Some(h) => PathBuf::from(h).join(".local/share/ocalc/history.json"),
        None => PathBuf::from(".ocalc-history.json"),
    }
}

// ---------------------------------------------------------------------------
// Directory creation
// ---------------------------------------------------------------------------

/// Create config + data (+ logs) + cache dirs. Returns the first I/O error.
pub fn ensure_dirs() -> std::io::Result<()> {
    for dir in [config_dir(), data_dir(), log_dir(), cache_dir()] {
        std::fs::create_dir_all(&dir)?;
    }
    Ok(())
}

/// Create the parent directory of `path` when it has one. Best-effort.
pub fn ensure_parent(path: &Path) {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(parent);
        }
    }
}

// ---------------------------------------------------------------------------
// Legacy migration (non-destructive copy + log)
// ---------------------------------------------------------------------------

/// Outcome of [`migrate_legacy`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MigrationReport {
    /// New settings file was populated from the legacy location.
    pub settings_migrated: bool,
    /// New history file was populated from the legacy location.
    pub history_migrated: bool,
}

/// Copy one legacy file to its new location when the new file does not exist
/// yet. Returns `true` when a copy happened.
fn migrate_one(legacy: &Path, fresh: &Path) -> bool {
    if fresh.exists() || !legacy.exists() {
        return false;
    }
    if legacy == fresh {
        return false;
    }
    ensure_parent(fresh);
    match std::fs::read(legacy) {
        Ok(bytes) => match std::fs::write(fresh, &bytes) {
            Ok(()) => true,
            Err(_) => false,
        },
        Err(_) => false,
    }
}

/// Migrate well-known legacy files into the XDG layout (explicit paths, for
/// tests and for hosts that resolve paths themselves).
pub fn migrate_legacy_to(
    legacy_settings: &Path,
    fresh_settings: &Path,
    legacy_history: &Path,
    fresh_history: &Path,
) -> MigrationReport {
    MigrationReport {
        settings_migrated: migrate_one(legacy_settings, fresh_settings),
        history_migrated: migrate_one(legacy_history, fresh_history),
    }
}

/// Migrate legacy files into this process's XDG layout. Best-effort: never
/// panics, never returns an error; the report tells what happened.
#[must_use]
pub fn migrate_legacy() -> MigrationReport {
    let report = migrate_legacy_to(
        &legacy_settings_path(),
        &settings_path(),
        &legacy_history_path(),
        &history_path(),
    );
    if report.settings_migrated {
        log_event(
            "config",
            &format!("migrated legacy settings → {}", settings_path().display()),
        );
    }
    if report.history_migrated {
        log_event(
            "config",
            &format!("migrated legacy history → {}", history_path().display()),
        );
    }
    report
}

// ---------------------------------------------------------------------------
// Production file logging (best-effort, never panics)
// ---------------------------------------------------------------------------

/// Append one timestamped line to the production log.
///
/// Format: `[<RFC3339 UTC>] [<tag>] <msg>\n`. Parent dirs are created on
/// demand. All failures are swallowed — logging must never crash the
/// calculator (callers that need feedback should also `eprintln!`).
pub fn log_event(tag: &str, msg: &str) {
    let path = log_file_path();
    ensure_parent(&path);
    let line = format!("[{}] [{tag}] {msg}\n", chrono::Utc::now().to_rfc3339());
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = f.write_all(line.as_bytes());
    }
}

/// Log a startup banner (version + resolved directories). Call once from each
/// binary entry point after [`ensure_dirs`] + [`migrate_legacy`].
pub fn log_startup(binary: &str, version: &str) {
    log_event(
        "startup",
        &format!(
            "{binary} v{version} starting (config={}, data={}, cache={})",
            config_dir().display(),
            data_dir().display(),
            cache_dir().display(),
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xdg_priority_config() {
        assert_eq!(
            config_dir_for(Some("/home/u"), Some("/xdg/cfg")),
            PathBuf::from("/xdg/cfg/ocalculator")
        );
        assert_eq!(
            config_dir_for(Some("/home/u"), None),
            PathBuf::from("/home/u/.config/ocalculator")
        );
        // Empty XDG falls back to HOME.
        assert_eq!(
            config_dir_for(Some("/home/u"), Some("  ")),
            PathBuf::from("/home/u/.config/ocalculator")
        );
        // No HOME at all → relative fallback (debug builds keep working).
        assert_eq!(config_dir_for(None, None), PathBuf::from("./ocalculator"));
    }

    #[test]
    fn xdg_priority_data_and_cache() {
        assert_eq!(
            data_dir_for(Some("/home/u"), Some("/xdg/data")),
            PathBuf::from("/xdg/data/ocalculator")
        );
        assert_eq!(
            data_dir_for(Some("/home/u"), None),
            PathBuf::from("/home/u/.local/share/ocalculator")
        );
        assert_eq!(
            cache_dir_for(Some("/home/u"), Some("/xdg/cache")),
            PathBuf::from("/xdg/cache/ocalculator")
        );
        assert_eq!(
            cache_dir_for(Some("/home/u"), None),
            PathBuf::from("/home/u/.cache/ocalculator")
        );
        assert_eq!(data_dir_for(None, None), PathBuf::from("./ocalculator"));
        assert_eq!(cache_dir_for(None, None), PathBuf::from("./ocalculator"));
    }

    #[test]
    fn file_names_under_dirs() {
        let cfg = PathBuf::from("/c");
        assert_eq!(
            cfg.join(SETTINGS_FILE_NAME),
            PathBuf::from("/c/settings.json")
        );
        let data = PathBuf::from("/d");
        assert_eq!(
            data.join(HISTORY_FILE_NAME),
            PathBuf::from("/d/history.json")
        );
        assert_eq!(
            data.join("logs").join(LOG_FILE_NAME),
            PathBuf::from("/d/logs/ocalculator.log")
        );
    }

    #[test]
    fn migration_copies_only_when_fresh_missing() {
        let dir = std::env::temp_dir().join("ocalc-paths-test-migrate");
        let _ = std::fs::remove_dir_all(&dir);
        let legacy_s = dir.join("legacy-s.json");
        let fresh_s = dir.join("sub/fresh-s.json");
        let legacy_h = dir.join("legacy-h.json");
        let fresh_h = dir.join("sub/fresh-h.json");
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        // No legacy → nothing happens.
        let r = migrate_legacy_to(&legacy_s, &fresh_s, &legacy_h, &fresh_h);
        assert_eq!(r, MigrationReport::default());
        // Legacy present, fresh missing → copy.
        std::fs::write(&legacy_s, b"{\"a\":1}").unwrap();
        std::fs::write(&legacy_h, b"{\"b\":2}").unwrap();
        let _ = std::fs::remove_file(&fresh_s);
        let _ = std::fs::remove_file(&fresh_h);
        let r = migrate_legacy_to(&legacy_s, &fresh_s, &legacy_h, &fresh_h);
        assert!(r.settings_migrated && r.history_migrated);
        assert_eq!(std::fs::read(&fresh_s).unwrap(), b"{\"a\":1}");
        assert_eq!(std::fs::read(&fresh_h).unwrap(), b"{\"b\":2}");
        // Fresh exists → never overwritten.
        std::fs::write(&legacy_s, b"{\"a\":2}").unwrap();
        let r = migrate_legacy_to(&legacy_s, &fresh_s, &legacy_h, &fresh_h);
        assert_eq!(r, MigrationReport::default());
        assert_eq!(std::fs::read(&fresh_s).unwrap(), b"{\"a\":1}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
