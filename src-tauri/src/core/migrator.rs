mod billfish_migrator;

use std::path::Path;

use serde::Deserialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    BillfishMigrator(#[from] billfish_migrator::Error),
}

trait Migrator<E>
where
    E: std::error::Error,
{
    fn migrate(path: &Path) -> Result<(), E>;
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MigrateFrom {
    Billfish,
}

pub fn migrate_from(path: &Path, from: MigrateFrom) -> Result<(), Error> {
    match from {
        MigrateFrom::Billfish => Ok(billfish_migrator::BillfishMigrator::migrate(path)?),
    }
}
