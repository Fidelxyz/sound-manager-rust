mod core;
mod response;

use core::database::{DatabaseEmitter, FolderId};
use core::migrator::{migrate_from, MigrateFrom, MigratorResult};
use core::player::{PlayerEmitter, PlayerState};
use core::{Database, EntryId, Filter, Player, TagId, WaveformGenerator};
use response::{to_serializable_map, Error};
use std::thread::spawn;

use std::option::Option;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::time::Duration;

use log::{debug, trace, warn};
use tauri::ipc::{Channel, InvokeResponseBody, Response};
use tauri::{
    App, AppHandle, Emitter, Manager, State, Theme, TitleBarStyle, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_window_state::{AppHandleExt, StateFlags};

struct AppData {
    database: RwLock<Option<Arc<Database<AppEmitter>>>>,
    player: RwLock<Player>,
    waveform_generator: Mutex<WaveformGenerator>,
    emitter: Arc<AppEmitter>,
}

struct AppEmitter {
    app: AppHandle,
    condvar: Condvar,
    state: Mutex<AppEmitterState>,
}

struct AppEmitterState {
    files_updated_immediate: bool,
    files_updated_delayed: bool,
    stopped: bool,
}

impl AppEmitter {
    fn new(app: AppHandle) -> Arc<Self> {
        let emitter = Arc::new(Self {
            app,
            condvar: Condvar::new(),
            state: Mutex::new(AppEmitterState {
                files_updated_immediate: false,
                files_updated_delayed: false,
                stopped: false,
            }),
        });

        let emitter_clone = emitter.clone();
        spawn(move || emitter_clone.run());

        emitter
    }

    fn run(&self) {
        debug!("Emitter thread started");

        let mut state = self.state.lock().unwrap();

        loop {
            state = self.condvar.wait(state).unwrap();

            if state.stopped {
                break;
            }

            if state.files_updated_immediate {
                drop(state);

                debug!("Emit: files_updated");
                self.app.emit("files_updated", ()).unwrap();

                state = self.state.lock().unwrap();
                state.files_updated_immediate = false;
                state.files_updated_delayed = false;
            } else if state.files_updated_delayed {
                (state, _) = self
                    .condvar
                    .wait_timeout_while(state, Duration::from_millis(500), |state| {
                        !state.files_updated_immediate && !state.stopped
                    })
                    .unwrap();

                if state.stopped {
                    break;
                }

                drop(state);

                debug!("Emit: files_updated");
                self.app.emit("files_updated", ()).unwrap();

                state = self.state.lock().unwrap();
                state.files_updated_immediate = false;
                state.files_updated_delayed = false;
            }
        }

        debug!("Emitter thread stopped");
    }

    fn stop(&self) {
        let mut state = self.state.lock().unwrap();
        state.stopped = true;
        self.condvar.notify_all();
    }
}

impl Drop for AppEmitter {
    fn drop(&mut self) {
        self.stop();
    }
}

impl PlayerEmitter for AppEmitter {
    fn on_player_state_updated(&self, state: PlayerState) {
        debug!("Emit: player_state_updated, {state:?}");
        self.app.emit("player_state_updated", state).unwrap();
    }
}

impl DatabaseEmitter for AppEmitter {
    fn on_files_updated(&self, immediate: bool) {
        trace!("on_files_updated, immediate = {immediate}");

        let mut state = self.state.lock().unwrap();
        if immediate {
            state.files_updated_immediate = true;
        } else {
            state.files_updated_delayed = true;
        }
        self.condvar.notify_all();
    }
}

// ========== Database ==========

macro_rules! get_database {
    ( $var:ident, $database:expr ) => {
        let $var = $database.read().unwrap();
        let $var = $var.as_ref().ok_or_else(|| Error::DatabaseNotOpen)?;
    };
}

macro_rules! get_data {
    ( $data:ident, $database:expr ) => {
        let $data = $database.data.read().unwrap();
    };
}

macro_rules! get_data_mut {
    ( $data:ident, $database:expr ) => {
        let mut $data = $database.data.write().unwrap();
    };
}

macro_rules! get_db {
    ( $db:ident, $database:expr ) => {
        let $db = $database.db.lock().unwrap();
    };
}

macro_rules! get_db_mut {
    ( $db:ident, $database:expr ) => {
        let mut $db = $database.db.lock().unwrap();
    };
}

#[tauri::command]
async fn open_database(path: String, state: State<'_, AppData>) -> Result<(), Error> {
    trace!("open_database: {path:?}");

    let mut database = state.database.write().unwrap();
    if let Some(database) = database.as_ref() {
        database.close();
    }
    database.replace(Database::open(path.into(), state.emitter.clone())?);

    state.player.write().unwrap().run();

    trace!("open_database done");
    Ok(())
}

#[tauri::command]
async fn create_database(path: String, state: State<'_, AppData>) -> Result<(), Error> {
    trace!("create_database: {path:?}");

    let mut database = state.database.write().unwrap();
    if let Some(database) = database.as_ref() {
        database.close();
    }
    database.replace(Database::create(path.into(), state.emitter.clone())?);

    state.player.write().unwrap().run();

    trace!("create_database done");
    Ok(())
}

#[tauri::command]
async fn close_database(state: State<'_, AppData>) -> Result<(), Error> {
    trace!("close_database");

    let database = state.database.write().unwrap().take();
    if let Some(database) = database {
        database.close();
    }

    state.player.read().unwrap().terminate();
    state.waveform_generator.lock().unwrap().set_source(None);

    trace!("close_database done");
    Ok(())
}

#[tauri::command]
async fn migrate_database(path: String, from_type: MigrateFrom) -> Result<MigratorResult, Error> {
    trace!("migrate_database: {path:?}");

    let path = PathBuf::from(path);
    let result = migrate_from(&path, &from_type)?;

    trace!("migrate_database done");
    Ok(result)
}

#[tauri::command]
async fn refresh(state: State<'_, AppData>) -> Result<(), Error> {
    trace!("refresh");

    get_database!(database, state.database);
    database.refresh()?;

    trace!("refresh done");
    Ok(())
}

#[tauri::command]
async fn get_entries(state: State<'_, AppData>) -> Result<Response, Error> {
    get_database!(database, state.database);
    get_data!(data, database);

    let entries = data.get_entries().values().collect::<Vec<_>>();
    let response = serde_json::to_string(&entries).unwrap();
    Ok(Response::new(response))
}

#[tauri::command]
async fn get_tags(state: State<'_, AppData>) -> Result<Response, Error> {
    get_database!(database, state.database);
    get_data!(data, database);

    let tags = data.get_tags();
    let response = serde_json::to_string(&tags).unwrap();
    Ok(Response::new(response))
}

#[tauri::command]
async fn get_folder(state: State<'_, AppData>) -> Result<Response, Error> {
    get_database!(database, state.database);
    get_data!(data, database);

    let folder = data.get_folders();
    let response = serde_json::to_string(&to_serializable_map(folder.iter()).unwrap()).unwrap();
    Ok(Response::new(response))
}

#[tauri::command]
async fn new_tag(name: String, state: State<'_, AppData>) -> Result<EntryId, Error> {
    trace!("new_tag: {name:?}");

    get_database!(database, state.database);
    get_data_mut!(data, database);
    get_db!(db, database);

    let tag_id = data.new_tag(name, &db)?;

    trace!("new_tag done");
    Ok(tag_id)
}

#[tauri::command]
async fn delete_tag(tag_id: TagId, state: State<'_, AppData>) -> Result<(), Error> {
    trace!("delete_tag: {tag_id:?}");

    get_database!(database, state.database);
    get_data_mut!(data, database);
    get_db!(db, database);

    data.delete_tag(tag_id, &db)?;

    trace!("delete_tag done");
    Ok(())
}

#[tauri::command]
async fn rename_tag(tag_id: TagId, name: String, state: State<'_, AppData>) -> Result<(), Error> {
    trace!("rename_tag: tag_id = {tag_id:?}, name = {name:?}");

    get_database!(database, state.database);
    get_data_mut!(data, database);
    get_db!(db, database);

    data.rename_tag(tag_id, name, &db)?;

    trace!("rename_tag done");
    Ok(())
}

#[tauri::command]
async fn reorder_tag(
    tag_id: TagId,
    new_parent_id: TagId,
    new_pos: i32,
    state: State<'_, AppData>,
) -> Result<(), Error> {
    trace!(
        "reorder_tag: tag_id = {tag_id:?}, new_parent_id = {new_parent_id:?}, new_pos = {new_pos:?}"
    );

    get_database!(database, state.database);
    get_data_mut!(data, database);
    get_db_mut!(db, database);

    data.reorder_tag(tag_id, new_parent_id, new_pos, &mut db)?;

    trace!("reorder_tag done");
    Ok(())
}

#[tauri::command]
async fn set_tag_color(tag_id: TagId, color: i32, state: State<'_, AppData>) -> Result<(), Error> {
    trace!("set_tag_color: tag_id = {tag_id:?}, color = {color:?}");

    get_database!(database, state.database);
    get_data_mut!(data, database);
    get_db!(db, database);

    data.set_tag_color(tag_id, color, &db)?;

    trace!("set_tag_color done");
    Ok(())
}

#[tauri::command]
async fn get_tags_for_entry(
    entry_id: EntryId,
    state: State<'_, AppData>,
) -> Result<Response, Error> {
    trace!("get_tags_for_entry: {entry_id:?}");

    get_database!(database, state.database);
    get_data!(data, database);

    let tags = data.get_tags_for_entry(entry_id);
    let response = serde_json::to_string(&tags).unwrap();

    trace!("get_tags_for_entry done");
    Ok(Response::new(response))
}

#[tauri::command]
async fn add_tag_for_entry(
    entry_id: EntryId,
    tag_id: TagId,
    state: State<'_, AppData>,
) -> Result<(), Error> {
    trace!("add_tag_for_entry: entry_id = {entry_id:?}, tag_id = {tag_id:?}");

    get_database!(database, state.database);
    get_data_mut!(data, database);
    get_db!(db, database);

    data.add_tag_for_entry(entry_id, tag_id, &db)?;

    trace!("add_tag_for_entry done");
    Ok(())
}

#[tauri::command]
async fn remove_tag_for_entry(
    entry_id: EntryId,
    tag_id: TagId,
    state: State<'_, AppData>,
) -> Result<(), Error> {
    trace!("remove_tag_for_entry: entry_id = {entry_id:?}, tag_id = {tag_id:?}");

    get_database!(database, state.database);
    get_data_mut!(data, database);
    get_db!(db, database);

    data.remove_tag_for_entry(entry_id, tag_id, &db)?;

    trace!("remove_tag_for_entry done");
    Ok(())
}

#[tauri::command]
async fn filter(filter: Filter, state: State<'_, AppData>) -> Result<Option<Vec<EntryId>>, Error> {
    trace!("filter: {filter:?}");

    get_database!(database, state.database);
    get_data!(data, database);

    let entry_ids = data.filter(&filter);

    trace!("filter done");
    Ok(entry_ids)
}

// ========== Waveform ==========

#[tauri::command]
async fn prepare_waveform(state: State<'_, AppData>) -> Result<u32, Error> {
    trace!("prepare_waveform");
    let waveform_generator = state.waveform_generator.lock().unwrap();
    let data_length = waveform_generator.prepare_waveform()?;
    trace!("prepare_waveform done");
    Ok(data_length)
}

#[tauri::command]
async fn request_waveform(
    state: State<'_, AppData>,
    channel: Channel<InvokeResponseBody>,
) -> Result<(), Error> {
    trace!("request_waveform");

    let waveform_generator = state.waveform_generator.lock().unwrap();
    waveform_generator.request_waveform(move |data| {
        debug!("send waveform data, len: {}", data.len());
        let response = InvokeResponseBody::Raw(data.to_vec());
        channel.send(response).unwrap();
    })?;

    trace!("request_waveform done");
    Ok(())
}

// ========== Player ==========

#[tauri::command]
async fn set_player_source(entry_id: EntryId, state: State<'_, AppData>) -> Result<(), Error> {
    trace!("set_player_source: {entry_id:?}");

    let path = {
        get_database!(database, state.database);
        get_data!(data, database);
        data.get_entry_path(entry_id).unwrap()
    };
    debug!("path: {}", path.display());

    state.player.write().unwrap().set_source(path.clone())?;

    state
        .waveform_generator
        .lock()
        .unwrap()
        .set_source(path.into());

    trace!("set_player_source done");
    Ok(())
}

#[tauri::command]
async fn seek(pos: f32, state: State<'_, AppData>) -> Result<(), Error> {
    trace!("seek: {pos:?}");

    state
        .player
        .read()
        .unwrap()
        .seek(Duration::from_secs_f32(pos))?;

    trace!("seek done");
    Ok(())
}

#[tauri::command]
async fn play(skip_silence: bool, state: State<'_, AppData>) -> Result<(), Error> {
    trace!("play: {skip_silence:?}");

    state.player.read().unwrap().play(skip_silence)?;

    trace!("play done");
    Ok(())
}

#[tauri::command]
async fn pause(state: State<'_, AppData>) -> Result<(), Error> {
    trace!("pause");

    state.player.read().unwrap().pause();

    trace!("pause done");
    Ok(())
}

#[tauri::command]
async fn stop(state: State<'_, AppData>) -> Result<(), Error> {
    trace!("stop");

    state.player.read().unwrap().stop();
    state.waveform_generator.lock().unwrap().set_source(None);

    trace!("stop done");
    Ok(())
}

#[tauri::command]
async fn set_volume(volume: f32, state: State<'_, AppData>) -> Result<(), Error> {
    trace!("set_volume: {volume:?}");

    state.player.write().unwrap().set_volume(volume)?;

    trace!("set_volume done");
    Ok(())
}

#[tauri::command]
async fn get_playing_pos(state: State<'_, AppData>) -> Result<f32, Error> {
    Ok(state.player.read().unwrap().get_pos())
}

// ========== Files ==========

#[tauri::command]
async fn import_file(path: String, force: bool, state: State<'_, AppData>) -> Result<(), Error> {
    trace!("import_file: {path:?}, force = {force:?}");

    get_database!(database, state.database);
    database.import_file(Path::new(&path), force)?;

    trace!("import_file done");
    Ok(())
}

#[tauri::command]
async fn delete_file(entry_id: EntryId, state: State<'_, AppData>) -> Result<(), Error> {
    trace!("delete_file: {entry_id:?}");

    get_database!(database, state.database);
    database.delete_file(entry_id)?;

    trace!("delete_file done");
    Ok(())
}

#[tauri::command]
async fn move_file(
    entry_id: EntryId,
    folder_id: FolderId,
    force: bool,
    state: State<'_, AppData>,
) -> Result<(), Error> {
    trace!("move_file: entry_id = {entry_id:?}, folder_id = {folder_id:?}, force = {force:?}");
    get_database!(database, state.database);

    database.move_file(entry_id, folder_id, force)?;

    trace!("move_file done");
    Ok(())
}

#[tauri::command]
async fn spot(
    entry_id: EntryId,
    save_path: Option<&str>,
    open_in_application: Option<&str>,
    force: bool,
    state: State<'_, AppData>,
) -> Result<(), Error> {
    trace!("spot: {entry_id:?}");

    get_database!(database, state.database);
    get_data!(data, database);

    data.spot(
        entry_id,
        save_path.map(Path::new),
        open_in_application.map(Path::new),
        force,
    )?;

    trace!("spot done");
    Ok(())
}

fn setup_state(app: &App) {
    let emitter = AppEmitter::new(app.handle().clone());
    app.manage(AppData {
        database: None.into(),
        player: Player::new(emitter.clone()).into(),
        waveform_generator: WaveformGenerator::new().into(),
        emitter,
    });
}

fn setup_window(app: &App) {
    let win_builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
        .title("Sound Manager")
        .inner_size(1600.0, 1000.0)
        .theme(Some(Theme::Dark))
        .disable_drag_drop_handler();

    // set transparent title bar only when building for macOS
    #[cfg(target_os = "macos")]
    let win_builder = win_builder.title_bar_style(TitleBarStyle::Transparent);

    let window = win_builder.build().unwrap();

    // set background color only when building for macOS
    #[cfg(target_os = "macos")]
    {
        use cocoa::appkit::{NSColor, NSWindow};
        use cocoa::base::{id, nil};

        let ns_window = window.ns_window().unwrap() as id;
        unsafe {
            let bg_color = NSColor::colorWithRed_green_blue_alpha_(nil, 0.12, 0.12, 0.12, 1.0);
            ns_window.setBackgroundColor_(bg_color);
        }
    }

    // persistant window state
    app.handle()
        .save_window_state(StateFlags::all())
        .unwrap_or_else(|e| warn!("Failed to save window state: {e:?}"));
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(
                    #[cfg(debug_assertions)]
                    log::LevelFilter::Debug,
                    #[cfg(not(debug_assertions))]
                    log::LevelFilter::Info,
                )
                .build(),
        )
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            setup_window(app);
            setup_state(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            open_database,
            create_database,
            close_database,
            migrate_database,
            refresh,
            get_entries,
            get_tags,
            get_folder,
            new_tag,
            delete_tag,
            rename_tag,
            reorder_tag,
            set_tag_color,
            get_tags_for_entry,
            add_tag_for_entry,
            remove_tag_for_entry,
            filter,
            prepare_waveform,
            request_waveform,
            set_player_source,
            seek,
            play,
            pause,
            stop,
            set_volume,
            get_playing_pos,
            import_file,
            delete_file,
            move_file,
            spot
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
