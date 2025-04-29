use super::{DatabaseData, Error, Result};

use std::fs::copy;
use std::path::Path;

use open;

impl DatabaseData {
    pub fn spot(
        &self,
        entry_id: i32,
        save_path: Option<&Path>,
        open_in_application: Option<&Path>,
        force: bool,
    ) -> Result<()> {
        let entry = self.get_entry(entry_id)?;
        let entry_path = self.to_absolute_path(&entry.path);

        // file to be opened in application
        let src_file_path = if let Some(save_path) = save_path {
            // save file to the new location
            let to_path = save_path.join(&entry.file_name);

            if !force && to_path.exists() {
                return Err(Error::FileAlreadyExists(to_path.to_string_lossy().into()));
            }

            copy(entry_path, &to_path)?;
            to_path.into_boxed_path()
        } else {
            entry_path
        };

        // open in application
        if let Some(open_in_application) = open_in_application {
            if open_in_application.exists() {
                open::with(
                    src_file_path.as_ref(),
                    open_in_application.to_string_lossy(),
                )?
            }
        }

        Ok(())
    }
}
