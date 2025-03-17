mod core;
mod response;

use core::{Database, Filter, Player, WaveformGenerator};
use response::Error;

use async_std::sync::RwLock;
use futures::try_join;
use log::{debug, trace};
use std::option::Option;
use std::path::PathBuf;
use std::time::Duration;

use tauri::ipc::{Channel, InvokeResponseBody, Response};
use tauri::{AppHandle, Emitter, Manager, State};

struct AppData {
    database: RwLock<Option<Database>>,
    player: RwLock<Player>,
    waveform_generator: RwLock<WaveformGenerator>,
}

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
        .replace(Database::open(path.into())?);
    state.player.write().await.run(move |player_state| {
        debug!("player_state_updated");
        app.emit("player_state_updated", player_state).unwrap();
    });

    trace!("open_database done");
    Ok(())
}

#[tauri::command]
async fn create_database(
    path: String,
    app: AppHandle,
    state: State<'_, AppData>,
) -> Result<(), Error> {
    trace!("create_database: {:?}", path);

    let path = PathBuf::from(path);
    state
        .database
        .write()
        .await
        .replace(Database::create(path.into())?);
    state.player.write().await.run(move |player_state| {
        debug!("player_state_updated");
        app.emit("player_state_updated", player_state).unwrap();
    });

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
async fn get_entries(state: State<'_, AppData>) -> Result<Response, Error> {
    let database = state.database.read().await;
    let entries = database
        .as_ref()
        .unwrap()
        .get_entries()
        .values()
        .collect::<Vec<_>>();
    let response = serde_json::to_string(&entries).unwrap();
    Ok(Response::new(response))
}

#[tauri::command]
async fn get_tags(state: State<'_, AppData>) -> Result<Response, Error> {
    let database = state.database.read().await;
    let tags = database.as_ref().unwrap().get_tags();
    let response = serde_json::to_string(&tags).unwrap();
    Ok(Response::new(response))
}

#[tauri::command]
async fn get_folder(state: State<'_, AppData>) -> Result<Response, Error> {
    let database = state.database.read().await;
    let folder = database.as_ref().unwrap().get_folder();
    let response = serde_json::to_string(folder).unwrap();
    Ok(Response::new(response))
}

#[tauri::command]
async fn new_tag(name: String, state: State<'_, AppData>) -> Result<i32, Error> {
    trace!("new_tag: {:?}", name);

    let mut database = state.database.write().await;
    let tag_id = database.as_mut().unwrap().new_tag(name)?;

    trace!("new_tag done");
    Ok(tag_id)
}

#[tauri::command]
async fn delete_tag(tag_id: i32, state: State<'_, AppData>) -> Result<(), Error> {
    trace!("delete_tag: {:?}", tag_id);

    let mut database = state.database.write().await;
    database.as_mut().unwrap().delete_tag(tag_id)?;

    trace!("delete_tag done");
    Ok(())
}

#[tauri::command]
async fn rename_tag(tag_id: i32, name: String, state: State<'_, AppData>) -> Result<(), Error> {
    trace!("rename_tag: tag_id = {:?}, name = {:?}", tag_id, name);

    let mut database = state.database.write().await;
    database.as_mut().unwrap().rename_tag(tag_id, name)?;

    trace!("rename_tag done");
    Ok(())
}

#[tauri::command]
async fn reorder_tag(tag_id: i32, new_pos: i32, state: State<'_, AppData>) -> Result<(), Error> {
    trace!(
        "reorder_tag: tag_id = {:?}, new_pos = {:?}",
        tag_id,
        new_pos
    );

    let mut database = state.database.write().await;
    database.as_mut().unwrap().reorder_tag(tag_id, new_pos)?;

    trace!("reorder_tag done");
    Ok(())
}

#[tauri::command]
async fn set_tag_color(tag_id: i32, color: i32, state: State<'_, AppData>) -> Result<(), Error> {
    trace!("set_tag_color: tag_id = {:?}, color = {:?}", tag_id, color);

    let mut database = state.database.write().await;
    database.as_mut().unwrap().set_tag_color(tag_id, color)?;

    trace!("set_tag_color done");
    Ok(())
}

#[tauri::command]
async fn get_tags_for_entry(entry_id: i32, state: State<'_, AppData>) -> Result<Response, Error> {
    trace!("get_tags_for_entry: {:?}", entry_id);

    let database = state.database.read().await;
    let tags = database.as_ref().unwrap().get_tags_for_entry(entry_id)?;
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

    let mut database = state.database.write().await;
    database
        .as_mut()
        .unwrap()
        .add_tag_for_entry(entry_id, tag_id)?;

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

    let mut database = state.database.write().await;
    database
        .as_mut()
        .unwrap()
        .remove_tag_for_entry(entry_id, tag_id)?;

    trace!("remove_tag_for_entry done");
    Ok(())
}

#[tauri::command]
async fn filter(filter: Filter, state: State<'_, AppData>) -> Result<Option<Vec<i32>>, Error> {
    trace!("filter: {:?}", filter);

    let database = state.database.read().await;
    let entry_ids = database.as_ref().unwrap().filter(filter);

    trace!("filter done");
    Ok(entry_ids)
}

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
    let expected_samples = waveform_generator.request_waveform(move |data| {
        debug!("send waveform data, len: {}", data.len());
        let response = InvokeResponseBody::Raw(data.to_vec());
        channel.send(response).unwrap();
    })?;

    trace!("request_waveform done");
    Ok(expected_samples)
}

#[tauri::command]
async fn set_player_source(entry_id: i32, state: State<'_, AppData>) -> Result<(), Error> {
    trace!("set_player_source: {:?}", entry_id);

    let path = state
        .database
        .read()
        .await
        .as_ref()
        .unwrap()
        .get_entry(entry_id)?
        .path
        .clone();
    debug!("path: {:?}", path);

    let mut player = state.player.write().await;
    let set_player_source = player.set_source(path.clone());

    let set_waveform_source = async {
        state.waveform_generator.write().await.set_source(path);
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
        .play(seek.map(|seek| Duration::from_secs_f32(seek)), skip_silence)?;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            app.manage(AppData {
                database: None.into(),
                player: Player::new().into(),
                waveform_generator: WaveformGenerator::new().into(),
            });
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            open_database,
            create_database,
            close_database,
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
            get_playing_pos
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
