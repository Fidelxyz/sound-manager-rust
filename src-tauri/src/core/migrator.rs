mod billfish_migrator;

use super::database::SQLITE_DB_PATH;

use std::fs::remove_file;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ========== Trait and Macros For Implementation ==========

#[derive(Error, Debug)]
enum MigratorImplError {
    #[error("Billfish migrator error: {0}")]
    BillfishMigrator(#[from] billfish_migrator::Error),
}

trait Migrator<E> {
    fn migrate(path: &Path, logger: &mut MigratorResult) -> Result<(), E>;
}

// ========== Error ==========

#[derive(Error, Debug)]
pub enum Error {
    #[error("database already exists: {0}")]
    DatabaseAlreadyExists(String),
}

// ========== MigratorResult ==========

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigratorResult {
    success: bool,
    logs: Vec<MigratorLog>,
}

#[derive(Serialize)]
#[serde(tag = "kind", content = "message")]
#[serde(rename_all = "camelCase")]
enum MigratorLog {
    Warn(String),
    Error(String),
}

impl MigratorResult {
    fn new() -> Self {
        MigratorResult {
            success: true,
            logs: Vec::new(),
        }
    }

    fn warn(&mut self, msg: String) {
        self.logs.push(MigratorLog::Warn(msg));
    }

    fn error(&mut self, msg: String) {
        self.success = false;
        self.logs.push(MigratorLog::Error(msg));
    }
}

#[macro_export]
macro_rules! migrator_warn {
    ( $logger:ident, $($arg:tt)+ ) => {{
        log::warn!($($arg)+);
        $logger.warn(format!($($arg)+));
    }};
}

// ========== Interfaces ==========

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MigrateFrom {
    Billfish,
}

pub fn migrate_from(path: &Path, from: &MigrateFrom) -> Result<MigratorResult, Error> {
    let mut result = MigratorResult::new();

    if path.join(SQLITE_DB_PATH).exists() {
        return Err(Error::DatabaseAlreadyExists(
            path.to_string_lossy().to_string(),
        ));
    }

    let impl_result: Result<_, MigratorImplError> = match from {
        MigrateFrom::Billfish => billfish_migrator::BillfishMigrator::migrate(path, &mut result)
            .map_err(std::convert::Into::into),
    };

    if let Err(err) = impl_result {
        result.error(err.to_string());
        remove_file(path.join(SQLITE_DB_PATH)).unwrap_or_else(|err| {
            migrator_warn!(result, "Failed to remove database file: {:?}", err);
        });
    }

    Ok(result)
}
