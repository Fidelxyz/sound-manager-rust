use super::{Database, DatabaseData, DatabaseEmitter, EntryId, Error, FolderId, Result};

use log::warn;
use std::fs::{copy, rename};
use std::path::Path;
use trash::delete;

use open;

impl<E> Database<E>
where
    E: DatabaseEmitter + Send + Sync + 'static,
{
    pub fn import_file(&self, path: &Path, force: bool) -> Result<()> {
        let mut data = self.data.write().unwrap();

        let file_name = path.file_name().unwrap();
        let dst_relative_path = Path::new(file_name);
        let dst_absolute_path = data.to_absolute_path(dst_relative_path);
        if !force && dst_absolute_path.exists() {
            return Err(Error::FileAlreadyExists(
                dst_absolute_path.to_string_lossy().into(),
            ));
        }

        copy(path, &dst_absolute_path)?;

        data.add_entries(&[dst_relative_path.into()], &self.db.lock().unwrap())?;
        self.emitter.on_files_updated(false);

        Ok(())
    }

    pub fn delete_file(&self, entry_id: EntryId) -> Result<()> {
        let mut data = self.data.write().unwrap();

        let path = data.to_absolute_path(&data.get_entry(entry_id).unwrap().path);
        delete(path)?;

        data.remove_entry(entry_id, &self.db.lock().unwrap())?;
        self.emitter.on_files_updated(false);

        Ok(())
    }

    pub fn move_file(&self, entry_id: EntryId, folder_id: FolderId, force: bool) -> Result<()> {
        let mut data = self.data.write().unwrap();

        let entry = data.get_entry(entry_id).unwrap();
        if entry.folder_id == folder_id {
            warn!("File {:?} is already in the target folder.", entry.path);
            return Ok(());
        }

        let new_path = data.to_absolute_path(
            &data
                .folders
                .get(&folder_id)
                .unwrap()
                .path
                .join(&entry.file_name),
        );
        if !force && new_path.exists() {
            return Err(Error::FileAlreadyExists(new_path.to_string_lossy().into()));
        }

        let old_path = data.to_absolute_path(&entry.path);
        rename(old_path, new_path)?;

        data.move_entry_to_folder(entry_id, folder_id, &mut self.db.lock().unwrap())?;
        self.emitter.on_files_updated(false);

        Ok(())
    }
}

impl DatabaseData {
    pub fn spot(
        &self,
        entry_id: EntryId,
        save_path: Option<&Path>,
        open_in_application: Option<&Path>,
        force: bool,
    ) -> Result<()> {
        let entry = self.get_entry(entry_id).unwrap();
        let entry_path = self.to_absolute_path(&entry.path);

        // file to be opened in application
        let src_file_path = if let Some(save_path) = save_path {
            // save file to the new location
            let to_path = save_path.join(&entry.file_name);

            if !force && to_path.exists() {
                return Err(Error::FileAlreadyExists(to_path.to_string_lossy().into()));
            }

            copy(entry_path, &to_path)?;
            to_path
        } else {
            entry_path
        };

        // open in application
        if let Some(open_in_application) = open_in_application {
            if open_in_application.exists() {
                open::with(src_file_path, open_in_application.to_string_lossy())?;
            }
        }

        Ok(())
    }
}
