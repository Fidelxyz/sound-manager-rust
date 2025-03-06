use super::player::get_format_reader;

use futures::{try_join, TryFutureExt};
use log::debug;
use std::path::Path;

use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use symphonia::core::meta::StandardTagKey;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("r2d2 error: {0}")]
    R2d2(#[from] r2d2::Error),
    #[error("symphonia error: {0}")]
    Symphonia(#[from] symphonia::core::errors::Error),
}

pub struct Entry {
    pub id: i32,
    pub path: Box<Path>,
    pub file_name: String,
    pub tag_ids: Vec<i32>,
    pub metadata: Option<Metadata>,
}

#[derive(Clone)]
pub struct Metadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: Option<f32>,
}

impl Entry {
    pub fn new(path: Box<Path>) -> Self {
        let file_name = path.file_name().unwrap().to_str().unwrap().to_string();

        Self {
            id: -1,
            path,
            file_name,
            tag_ids: Vec::new(),
            metadata: None,
        }
    }

    pub async fn read(
        &mut self,
        db_conn_pool: &Pool<SqliteConnectionManager>,
    ) -> Result<(), Error> {
        //debug!("Reading entries: {:?}", self.path);

        let future_read_metadata = self.read_metadata().err_into::<Error>();
        let future_query_database = self.query_database(db_conn_pool.get()?).err_into::<Error>();

        let (metadata, (id, tag_ids)) = try_join!(future_read_metadata, future_query_database)?;
        self.metadata = Some(metadata);
        self.id = id;
        self.tag_ids = tag_ids;
        Ok(())
    }

    async fn read_metadata(&self) -> Result<Metadata, symphonia::core::errors::Error> {
        let mut ret = Metadata {
            title: None,
            artist: None,
            album: None,
            duration: None,
        };

        // Read metadata
        let mut format = get_format_reader(&self.path)?;

        let mut metadata = format.metadata();
        if let Some(metadata) = metadata.skip_to_latest() {
            for tag in metadata.tags() {
                match tag.std_key {
                    Some(StandardTagKey::TrackTitle) => ret.title = Some(tag.value.to_string()),
                    Some(StandardTagKey::Artist) => ret.artist = Some(tag.value.to_string()),
                    Some(StandardTagKey::Album) => ret.album = Some(tag.value.to_string()),
                    _ => {}
                }
            }
        }

        if let Some(track) = format.default_track() {
            let params = &track.codec_params;
            if let (Some(n_frames), Some(time_base)) = (params.n_frames, params.time_base) {
                let time = time_base.calc_time(n_frames);
                ret.duration = Some(time.seconds as f32 + time.frac as f32);
            }
        }

        Ok(ret)
    }

    async fn query_database(
        &self,
        db: PooledConnection<SqliteConnectionManager>,
    ) -> Result<(i32, Vec<i32>), rusqlite::Error> {
        let ret_id;
        let mut ret_tag_ids = Vec::new();

        // Query from database
        match db.query_row(
            "SELECT id FROM entries WHERE file_path = ?",
            &[&self.path.to_str().unwrap()],
            |row| row.get(0),
        ) {
            Ok(id) => {
                // Entry already exists
                ret_id = id;

                // Read tags
                let mut stmt = db
                    .prepare("SELECT tag_id FROM entry_tag WHERE entry_id = ?")
                    .unwrap();
                let mut rows = stmt.query([&self.id]).unwrap();

                while let Some(row) = rows.next().unwrap() {
                    let tag_id = row.get(0).unwrap();
                    ret_tag_ids.push(tag_id);
                }
            }
            Err(_) => {
                // Create new entry in database
                let mut stmt = db
                    .prepare("INSERT INTO entries (file_path) VALUES (?)")
                    .unwrap();
                stmt.execute(&[&self.path.to_str().unwrap()]).unwrap();

                ret_id = db.last_insert_rowid() as i32;
            }
        };

        Ok((ret_id, ret_tag_ids))
    }
}
