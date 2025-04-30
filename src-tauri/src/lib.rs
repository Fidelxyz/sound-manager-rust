mod core;
mod response;

use core::database::DatabaseEmitter;
use core::migrator::{migrate_from, MigrateFrom};
use core::player::{PlayerEmitter, PlayerState};
use core::{Database, Filter, Player, WaveformGenerator};
use response::Error;

use async_std::sync::RwLock;
use futures::try_join;
use log::{debug, trace};
use std::option::Option;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tauri::ipc::{Channel, InvokeResponseBody, Response};
use tauri::{
    AppHandle, Emitter, Manager, State, Theme, TitleBarStyle, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_window_state::{AppHandleExt, StateFlags};

struct AppData {
    database: RwLock<Option<Database>>,
    player: RwLock<Player>,
    waveform_generator: RwLock<WaveformGenerator>,
    emitter: AppEmitter,
}

#[derive(Clone)]
struct AppEmitter {
    app: AppHandle,
}

impl AppEmitter {
    fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl PlayerEmitter for AppEmitter {
    fn on_player_state_updated(&self, state: PlayerState) {
        trace!("Emit: player_state_updated, {:?}", state);
        self.app.emit("player_state_updated", state).unwrap();
    }
}

impl DatabaseEmitter for AppEmitter {
    fn on_files_updated(&self) {
        trace!("Emit: files_updated");
        self.app.emit("files_updated", ()).unwrap();
    }
}

// ========== Database ==========

#[tauri::command]
async fn open_database(
    path: String,
    app: AppHandle,
    state: State<'_, AppData>,
) -> Result<(), Error> {
    trace!("open_database: {:?}", path);

    let path = PathBuf::from(path);
    state
        .database
        .write()
        .await
        .replace(Database::open(path.into(), AppEmitter::new(app))?);
    state.player.write().await.run();

    trace!("open_database done");
    Ok(())
}

#[tauri::command]
async fn create_database(path: String, state: State<'_, AppData>) -> Result<(), Error> {
    trace!("create_database: {:?}", path);

    let path = PathBuf::from(path);
    state
        .database
        .write()
        .await
        .replace(Database::create(path.into(), state.emitter.clone())?);
    state.player.write().await.run();

    trace!("create_database done");
    Ok(())
}

#[tauri::command]
async fn close_database(state: State<'_, AppData>) -> Result<(), Error> {
    trace!("close_database");

    state.database.write().await.take();
    state.player.read().await.stop();

    trace!("close_database done");
    Ok(())
}

#[tauri::command]
async fn migrate_database(
    path: String,
    from_type: MigrateFrom,
    state: State<'_, AppData>,
    app: AppHandle,
) -> Result<(), Error> {
    trace!("migrate_database: {:?}", path);

    let path = PathBuf::from(path);
    migrate_from(&path, from_type)?;

    state
        .database
        .write()
        .await
        .replace(Database::open(path.into(), AppEmitter::new(app))?);
    state.player.write().await.run();

    trace!("migrate_database done");
    Ok(())
}

#[tauri::command]
async fn refresh(state: State<'_, AppData>) -> Result<(), Error> {
    trace!("refresh");

    let database = state.database.read().await;
    let database = database.as_ref().ok_or_else(|| Error::DatabaseNotOpen)?;
    database.refresh()?;

    trace!("refresh done");
    Ok(())
}

#[tauri::command]
async fn get_entries(state: State<'_, AppData>) -> Result<Response, Error> {
    let database = state.database.read().await;
    let database = database.as_ref().ok_or_else(|| Error::DatabaseNotOpen)?;
    let data = database.data.read().unwrap();
    let entries = data.get_entries().values().collect::<Vec<_>>();
    let response = serde_json::to_string(&entries).unwrap();
    Ok(Response::new(response))
}

#[tauri::command]
async fn get_tags(state: State<'_, AppData>) -> Result<Response, Error> {
    let database = state.database.read().await;
    let database = database.as_ref().ok_or_else(|| Error::DatabaseNotOpen)?;
    let data = database.data.read().unwrap();
    let tags = data.get_tags();
    let response = serde_json::to_string(&tags).unwrap();
    Ok(Response::new(response))
}

#[tauri::command]
async fn get_folder(state: State<'_, AppData>) -> Result<Response, Error> {
    let database = state.database.read().await;
    let database = database.as_ref().ok_or_else(|| Error::DatabaseNotOpen)?;
    let data = database.data.read().unwrap();
    let folder = data.get_folders();
    let response = serde_json::to_string(folder).unwrap();
    Ok(Response::new(response))
}

#[tauri::command]
async fn new_tag(name: String, state: State<'_, AppData>) -> Result<i32, Error> {
    trace!("new_tag: {:?}", name);

    let database = state.database.read().await;
    let database = database.as_ref().ok_or_else(|| Error::DatabaseNotOpen)?;
    let mut data = database.data.write().unwrap();
    let tag_id = data.new_tag(name, &database.db.lock().unwrap())?;

    trace!("new_tag done");
    Ok(tag_id)
}

#[tauri::command]
async fn delete_tag(tag_id: i32, state: State<'_, AppData>) -> Result<(), Error> {
    trace!("delete_tag: {:?}", tag_id);

    let database = state.database.read().await;
    let database = database.as_ref().ok_or_else(|| Error::DatabaseNotOpen)?;
    let mut data = database.data.write().unwrap();
    data.delete_tag(tag_id, &database.db.lock().unwrap())?;

    trace!("delete_tag done");
    Ok(())
}

#[tauri::command]
async fn rename_tag(tag_id: i32, name: String, state: State<'_, AppData>) -> Result<(), Error> {
    trace!("rename_tag: tag_id = {:?}, name = {:?}", tag_id, name);

    let database = state.database.read().await;
    let database = database.as_ref().ok_or_else(|| Error::DatabaseNotOpen)?;
    let mut data = database.data.write().unwrap();
    data.rename_tag(tag_id, name, &database.db.lock().unwrap())?;

    trace!("rename_tag done");
    Ok(())
}

#[tauri::command]
async fn reorder_tag(
    tag_id: i32,
    new_parent_id: i32,
    new_pos: i32,
    state: State<'_, AppData>,
) -> Result<(), Error> {
    trace!(
        "reorder_tag: tag_id = {:?}, new_parent_id = {:?}, new_pos = {:?}",
        tag_id,
        new_parent_id,
        new_pos
    );

    let database = state.database.read().await;
    let database = database.as_ref().ok_or_else(|| Error::DatabaseNotOpen)?;
    let mut data = database.data.write().unwrap();
    data.reorder_tag(
        tag_id,
        new_parent_id,
        new_pos,
        &mut database.db.lock().unwrap(),
    )?;

    trace!("reorder_tag done");
    Ok(())
}

#[tauri::command]
async fn set_tag_color(tag_id: i32, color: i32, state: State<'_, AppData>) -> Result<(), Error> {
    trace!("set_tag_color: tag_id = {:?}, color = {:?}", tag_id, color);

    let database = state.database.read().await;
    let database = database.as_ref().ok_or_else(|| Error::DatabaseNotOpen)?;
    let mut data = database.data.write().unwrap();
    data.set_tag_color(tag_id, color, &database.db.lock().unwrap())?;

    trace!("set_tag_color done");
    Ok(())
}

#[tauri::command]
async fn get_tags_for_entry(entry_id: i32, state: State<'_, AppData>) -> Result<Response, Error> {
    trace!("get_tags_for_entry: {:?}", entry_id);

    let database = state.database.read().await;
    let database = database.as_ref().ok_or_else(|| Error::DatabaseNotOpen)?;
    let data = database.data.read().unwrap();
    let tags = data.get_tags_for_entry(entry_id)?;
    let response = serde_json::to_string(&tags).unwrap();

    trace!("get_tags_for_entry done");
    Ok(Response::new(response))
}

#[tauri::command]
async fn add_tag_for_entry(
    entry_id: i32,
    tag_id: i32,
    state: State<'_, AppData>,
) -> Result<(), Error> {
    trace!(
        "add_tag_for_entry: entry_id = {:?}, tag_id = {:?}",
        entry_id,
        tag_id
    );

    let database = state.database.read().await;
    let database = database.as_ref().ok_or_else(|| Error::DatabaseNotOpen)?;
    let mut data = database.data.write().unwrap();
    data.add_tag_for_entry(entry_id, tag_id, &database.db.lock().unwrap())?;

    trace!("add_tag_for_entry done");
    Ok(())
}

#[tauri::command]
async fn remove_tag_for_entry(
    entry_id: i32,
    tag_id: i32,
    state: State<'_, AppData>,
) -> Result<(), Error> {
    trace!(
        "remove_tag_for_entry: entry_id = {:?}, tag_id = {:?}",
        entry_id,
        tag_id
    );

    let database = state.database.read().await;
    let database = database.as_ref().ok_or_else(|| Error::DatabaseNotOpen)?;
    let mut data = database.data.write().unwrap();
    data.remove_tag_for_entry(entry_id, tag_id, &database.db.lock().unwrap())?;

    trace!("remove_tag_for_entry done");
    Ok(())
}

#[tauri::command]
async fn filter(filter: Filter, state: State<'_, AppData>) -> Result<Option<Vec<i32>>, Error> {
    trace!("filter: {:?}", filter);

    let database = state.database.read().await;
    let database = database.as_ref().ok_or_else(|| Error::DatabaseNotOpen)?;
    let data = database.data.read().unwrap();
    let entry_ids = data.filter(filter);

    trace!("filter done");
    Ok(entry_ids)
}

// ========== Waveform ==========

#[tauri::command]
async fn prepare_waveform(state: State<'_, AppData>) -> Result<u32, Error> {
    trace!("prepare_waveform");
    let waveform_generator = state.waveform_generator.read().await;
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

    let waveform_generator = state.waveform_generator.read().await;
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
async fn set_player_source(entry_id: i32, state: State<'_, AppData>) -> Result<(), Error> {
    trace!("set_player_source: {:?}", entry_id);

    let path = {
        let database = state.database.read().await;
        let database = database.as_ref().ok_or_else(|| Error::DatabaseNotOpen)?;
        let data = database.data.read().unwrap();
        data.to_absolute_path(&data.get_entry(entry_id)?.path)
    };
    debug!("path: {:?}", path);

    let mut player = state.player.write().await;
    let set_player_source = player.set_source(&path);

    let set_waveform_source = async {
        state
            .waveform_generator
            .write()
            .await
            .set_source(path.clone());
        Ok(())
    };

    try_join!(set_player_source, set_waveform_source)?;

    trace!("set_player_source done");
    Ok(())
}

#[tauri::command]
async fn play(
    seek: Option<f32>,
    skip_silence: bool,
    state: State<'_, AppData>,
) -> Result<(), Error> {
    trace!("play: {:?}", seek);

    state
        .player
        .read()
        .await
        .play(seek.map(Duration::from_secs_f32), skip_silence)?;

    trace!("play done");
    Ok(())
}

#[tauri::command]
async fn pause(state: State<'_, AppData>) -> Result<(), Error> {
    trace!("pause");

    state.player.read().await.pause();

    trace!("pause done");
    Ok(())
}

#[tauri::command]
async fn set_volume(volume: f32, state: State<'_, AppData>) -> Result<(), Error> {
    trace!("set_volume: {:?}", volume);

    state.player.write().await.set_volume(volume)?;

    trace!("set_volume done");
    Ok(())
}

#[tauri::command]
async fn get_playing_pos(state: State<'_, AppData>) -> Result<f32, Error> {
    Ok(state.player.read().await.get_pos())
}

// ========== Spotter ==========

#[tauri::command]
async fn spot(
    entry_id: i32,
    save_path: Option<&str>,
    open_in_application: Option<&str>,
    force: bool,
    state: State<'_, AppData>,
) -> Result<(), Error> {
    trace!("spot: {:?}", entry_id);

    let database = state.database.read().await;
    let database = database.as_ref().ok_or_else(|| Error::DatabaseNotOpen)?;
    let data = database.data.read().unwrap();

    data.spot(
        entry_id,
        save_path.map(Path::new),
        open_in_application.map(Path::new),
        force,
    )?;

    trace!("spot done");
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let win_builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                .title("Sound Manager")
                .inner_size(1600.0, 1000.0)
                .theme(Some(Theme::Dark));

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
                    let bg_color =
                        NSColor::colorWithRed_green_blue_alpha_(nil, 0.12, 0.12, 0.12, 1.0);
                    ns_window.setBackgroundColor_(bg_color);
                }
            }

            app.handle().save_window_state(StateFlags::all())?;

            let emitter = AppEmitter::new(app.handle().clone());
            app.manage(AppData {
                database: None.into(),
                player: Player::new(emitter.clone()).into(),
                waveform_generator: WaveformGenerator::new().into(),
                emitter,
            });
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
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
            play,
            pause,
            set_volume,
            get_playing_pos,
            spot
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
