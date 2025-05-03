mod billfish_migrator;

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

#[derive(Serialize)]
#[serde(tag = "kind", content = "message")]
#[serde(rename_all = "camelCase")]
enum MigratorLog {
    Warn(String),
    Error(String),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigratorResult {
    success: bool,
    logs: Vec<MigratorLog>,
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
macro_rules! warn {
    ($logger:ident, $($arg:tt)+) => {{
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

pub fn migrate_from(path: &Path, from: &MigrateFrom) -> MigratorResult {
    let mut result = MigratorResult::new();

    let impl_result: Result<_, MigratorImplError> = match from {
        MigrateFrom::Billfish => billfish_migrator::BillfishMigrator::migrate(path, &mut result)
            .map_err(std::convert::Into::into),
    };

    if let Err(err) = impl_result {
        result.error(err.to_string());
    }

    result
}
