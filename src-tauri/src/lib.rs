mod core;
mod response;

use core::{Database, Player, WaveformGenerator};
use response::Error;

use async_std::sync::RwLock;
use futures::{try_join, TryFutureExt};
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
async fn create_database(path: String, state: State<'_, AppData>) -> Result<(), Error> {
    trace!("create_database: {:?}", path);

    let path = PathBuf::from(path);
    state
        .database
        .write()
        .await
        .replace(Database::create(path.into())?);

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
    let entries = &database.as_ref().unwrap().entries;
    let response = serde_json::to_string(entries).unwrap();
    Ok(Response::new(response))
}

#[tauri::command]
async fn get_tags(state: State<'_, AppData>) -> Result<Response, Error> {
    let database = state.database.read().await;
    let tags = &database.as_ref().unwrap().tags;
    let response = serde_json::to_string(tags).unwrap();
    Ok(Response::new(response))
}

#[tauri::command]
async fn get_folder(state: State<'_, AppData>) -> Result<Response, Error> {
    let database = state.database.read().await;
    let folder = &database.as_ref().unwrap().folder;
    let response = serde_json::to_string(folder).unwrap();
    Ok(Response::new(response))
}

#[tauri::command]
async fn prepare_waveform(state: State<'_, AppData>) -> Result<u32, Error> {
    debug!("prepare_waveform");
    let waveform_generator = state.waveform_generator.read().await;
    let data_length = waveform_generator.prepare_waveform()?;
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
        .get_entry(entry_id)
        .ok_or_else(|| Error::EntryNotFound(entry_id))?
        .path
        .clone();
    debug!("path: {:?}", path);

    let mut player = state.player.write().await;
    let set_player_source = player.set_source(path.clone()).err_into::<Error>();

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
    // TODO: BUG HERE

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
