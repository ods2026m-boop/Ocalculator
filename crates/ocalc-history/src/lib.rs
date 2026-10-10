//! `ocalc-history`: calculation history, JSON storage, search.
//!
//! Also owns the XDG application directories ([`paths`]): config (`settings`),
//! data (`history`, `logs`) and cache locations shared by the CLI and the
//! desktop frontend, plus legacy migration and best-effort file logging.

mod error;
mod paths;
mod store;

pub use error::HistoryError;
pub use paths::{
    cache_dir, cache_dir_for, config_dir, config_dir_for, data_dir, data_dir_for, ensure_dirs,
    ensure_parent, history_path, legacy_history_path, legacy_settings_path, log_dir, log_event,
    log_file_path, log_startup, migrate_legacy, migrate_legacy_to, settings_path, MigrationReport,
    APP_DIR_NAME, HISTORY_FILE_NAME, LOG_FILE_NAME, SETTINGS_FILE_NAME,
};
pub use store::{HistoryEntry, HistoryStore, HISTORY_FORMAT_VERSION};
