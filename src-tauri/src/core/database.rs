use super::Entry;
use super::Folder;

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
    #[error("database not found: {0}")]
    DatabaseNotFound(String),
    #[error("database already exists: {0}")]
    DatabaseAlreadyExists(String),
    #[error("entry not found: {0}")]
    EntryNotFound(i32),
    #[error("tag not found: {0}")]
    TagNotFound(i32),
    #[error("tag already exists: {0}")]
    TagAlreadyExists(String),
    #[error("tag {0} not found for entry {1}")]
    TagNotFoundForEntry(i32, i32),
    #[error("tag {0} already exists for entry {1}")]
    TagAlreadyExistsForEntry(i32, i32),
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("r2d2 error: {0}")]
    R2d2(#[from] r2d2::Error),
    #[error("symphonia error: {0}")]
    Symphonia(#[from] symphonia::core::errors::Error),
}

#[derive(Clone)]
pub struct Tag {
    pub id: i32,
    pub name: String,
}

pub struct Database {
    pub base_path: Box<Path>,

    pub folder: Folder,
    pub entries: Vec<Entry>,

    db_conn_pool: Pool<SqliteConnectionManager>,
}

impl Database {
    pub fn open(base_path: Box<Path>) -> Result<Database, Error> {
        let database_file = base_path.join(".soundmanager.db");
        if !database_file.try_exists().unwrap() {
            // if database file does not exist
            return Err(Error::DatabaseNotFound(
                base_path.to_str().unwrap().to_owned(),
            ));
        }

        let manager = SqliteConnectionManager::file(database_file);
        let db_conn_pool = r2d2::Pool::new(manager)?;

        let (mut entries, folder) = block_on(read_dir(base_path.clone(), &db_conn_pool));
        read_entries(&mut entries, &db_conn_pool);

        Ok(Database {
            base_path,
            folder,
            entries,
            db_conn_pool,
        })
    }

    pub fn create(base_path: Box<Path>) -> Result<Database, Error> {
        let database_file = base_path.join(".soundmanager.db");
        if database_file.try_exists().unwrap() {
            // if database file already exists
            return Err(Error::DatabaseAlreadyExists(
                base_path.to_str().unwrap().to_owned(),
            ));
        }

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
                FOREIGN KEY (entry_id) REFERENCES entries(id) ON DELETE CASCADE,
                FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
            )",
            (),
        )?;

        let (mut entries, folder) = block_on(read_dir(base_path.clone(), &db_conn_pool));
        read_entries(&mut entries, &db_conn_pool);

        Ok(Database {
            base_path,
            entries,
            folder,
            db_conn_pool,
        })
    }

    pub fn get_entry(&self, entry_id: i32) -> Result<&Entry, Error> {
        self.entries
            .iter()
            .find(|entry| entry.id == entry_id)
            .ok_or_else(|| Error::EntryNotFound(entry_id))
    }

    pub fn new_tag(&self, name: String) -> Result<Tag, Error> {
        let db = self.db_conn_pool.get()?;

        let mut stmt = db.prepare("SELECT 1 FROM tags WHERE name = ?")?;
        if stmt.exists(&[&name])? {
            return Err(Error::TagAlreadyExists(name));
        }

        let mut stmt = db.prepare("INSERT INTO tags (name) VALUES (?)")?;
        let id = stmt.insert(&[&name])?;

        Ok(Tag {
            id: id as i32,
            name,
        })
    }

    pub fn delete_tag(&self, tag_id: i32) -> Result<(), Error> {
        let db = self.db_conn_pool.get()?;

        let mut stmt = db.prepare("SELECT 1 FROM tags WHERE id = ?")?;
        if !stmt.exists(&[&tag_id])? {
            return Err(Error::TagNotFound(tag_id));
        }

        db.execute("DELETE FROM tags WHERE id = ?", &[&tag_id])?;

        Ok(())
    }

    pub fn rename_tag(&self, tag_id: i32, name: String) -> Result<(), Error> {
        let db = self.db_conn_pool.get()?;

        let mut stmt = db.prepare("SELECT 1 FROM tags WHERE id = ?")?;
        if !stmt.exists(&[&tag_id])? {
            return Err(Error::TagNotFound(tag_id));
        }

        let mut stmt = db.prepare("SELECT 1 FROM tags WHERE name = ?")?;
        if stmt.exists(&[&name])? {
            return Err(Error::TagAlreadyExists(name));
        }

        db.execute("UPDATE tags SET name = ? WHERE id = ?", (&name, &tag_id))?;

        Ok(())
    }

    pub fn get_tags(&self) -> Result<Vec<Tag>, Error> {
        let db = self.db_conn_pool.get()?;

        let mut stmt = db.prepare("SELECT id, name FROM tags")?;
        let tags = stmt
            .query_map([], |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tags)
    }

    pub fn add_tag_for_entry(&self, entry_id: i32, tag_id: i32) -> Result<(), Error> {
        let entry = self
            .entries
            .iter()
            .find(|entry| entry.id == entry_id)
            .ok_or(Error::EntryNotFound(entry_id))?;

        entry.add_tag(tag_id, self.db_conn_pool.get()?)?;

        Ok(())
    }

    pub fn get_tags_for_entry(&self, entry_id: i32) -> Result<Vec<Tag>, Error> {
        let db = self.db_conn_pool.get()?;

        let mut stmt = db.prepare(
            "SELECT tags.id, tags.name FROM tags
            JOIN entry_tag ON tags.id = entry_tag.tag_id
            WHERE entry_tag.entry_id = ?",
        )?;
        let tags = stmt
            .query_map([&entry_id], |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tags)
    }

    pub fn remove_tag_for_entry(&self, entry_id: i32, tag_id: i32) -> Result<(), Error> {
        let entry = self
            .entries
            .iter()
            .find(|entry| entry.id == entry_id)
            .ok_or(Error::EntryNotFound(entry_id))?;

        entry.remove_tag(tag_id, self.db_conn_pool.get()?)
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
