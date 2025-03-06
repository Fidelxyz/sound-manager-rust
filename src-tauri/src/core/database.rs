use super::entry::Entry;
use super::folder::Folder;
use super::tag::Tag;

use std::path::Path;
use std::result::Result;

use futures::executor::block_on;
use futures::future::join_all;
use futures::TryFutureExt;
use log::debug;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("file not found: {0}")]
    NotFound(#[from] std::io::Error),
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("r2d2 error: {0}")]
    R2d2(#[from] r2d2::Error),
}

pub struct Database {
    pub base_path: Box<Path>,

    pub folder: Folder,
    pub entries: Vec<Entry>,
    pub tags: Vec<Tag>,

    db_conn_pool: Pool<SqliteConnectionManager>,
}

impl Database {
    pub fn open(base_path: Box<Path>) -> Result<Database, Error> {
        let database_file = base_path.join(".soundmanager.db");
        if !database_file.try_exists().unwrap() {
            // if database file does not exist
            return Err(Error::from(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                base_path.to_str().unwrap().to_owned(),
            )));
        }

        let manager = SqliteConnectionManager::file(database_file);
        let db_conn_pool = r2d2::Pool::new(manager)?;

        let db = db_conn_pool.get()?;
        let tags = Tag::read_all(db);

        let (mut entries, folder) = block_on(read_dir(base_path.clone(), &db_conn_pool));
        read_entries(&mut entries, &db_conn_pool);

        Ok(Database {
            base_path,
            folder,
            entries,
            tags,
            db_conn_pool,
        })
    }

    pub fn create(base_path: Box<Path>) -> Result<Database, Error> {
        let database_file = base_path.join(".soundmanager.db");
        let manager = SqliteConnectionManager::file(database_file);
        let db_conn_pool = r2d2::Pool::new(manager)?;

        let db = db_conn_pool.get()?;
        db.execute(
            "CREATE TABLE IF NOT EXISTS entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_path TEXT NOT NULL
            )",
            (),
        )?;
        db.execute(
            "CREATE TABLE IF NOT EXISTS tags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL
            )",
            (),
        )?;
        db.execute(
            "CREATE TABLE IF NOT EXISTS entry_tag (
                entry_id INTEGER NOT NULL,
                tag_id INTEGER NOT NULL,
                PRIMARY KEY (entry_id, tag_id),
                FOREIGN KEY (entry_id) REFERENCES entries(id),
                FOREIGN KEY (tag_id) REFERENCES tags(id)
            )",
            (),
        )?;

        let (mut entries, folder) = block_on(read_dir(base_path.clone(), &db_conn_pool));
        read_entries(&mut entries, &db_conn_pool);

        Ok(Database {
            base_path,
            entries,
            folder,
            tags: Vec::new(),
            db_conn_pool,
        })
    }

    pub fn get_entry(&self, entry_id: i32) -> Option<&Entry> {
        self.entries.iter().find(|entry| entry.id == entry_id)
    }
}

async fn read_dir(
    path: Box<Path>,
    db_conn_pool: &Pool<SqliteConnectionManager>,
) -> (Vec<Entry>, Folder) {
    debug!("Reading directory: {:?}", path);

    let mut futures = Vec::new();
    let mut entries = Vec::new();

    for dir_entry in std::fs::read_dir(&path).unwrap() {
        let dir_entry = dir_entry.expect("Error reading directory entry");

        // Skip hidden files and folders
        if dir_entry.file_name().to_str().unwrap().starts_with(".") {
            continue;
        }

        if dir_entry.file_type().unwrap().is_dir() {
            let path = dir_entry.path().as_path().into();
            futures.push(read_dir(path, db_conn_pool));
        } else {
            let path = dir_entry.path();
            let ext = path.extension().unwrap().to_str().unwrap();
            if ext == "wav" || ext == "mp3" || ext == "flac" || ext == "ogg" {
                entries.push(Entry::new(dir_entry.path().as_path().into()));
            }
        }
    }

    let result = join_all(futures).await;
    let mut sub_folders = Vec::new();

    for (sub_entries, sub_folder) in result {
        entries.extend(sub_entries);
        sub_folders.push(sub_folder);
    }

    let name = path.file_name().unwrap().to_str().unwrap().to_string();

    (
        entries,
        Folder {
            path,
            name,
            sub_folders,
        },
    )
}

fn read_entries(entries: &mut Vec<Entry>, db_conn_pool: &Pool<SqliteConnectionManager>) {
    debug!("Reading entries");

    let mut futures = Vec::new();

    for entry in entries {
        futures.push(
            entry
                .read(db_conn_pool)
                .map_err(|e| debug!("Error reading entry: {}", e)),
        );
    }

    let _ = block_on(join_all(futures));
}
