// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use simplelog::{ColorChoice, TerminalMode};

fn main() {
    #[cfg(debug_assertions)]
    let log_level = log::LevelFilter::Trace;

    #[cfg(not(debug_assertions))]
    let log_level = log::LevelFilter::Info;

    let log_config = simplelog::ConfigBuilder::new()
        // .add_filter_allow_str("sound_manager_rust_lib")
        .build();

    simplelog::TermLogger::init(
        log_level,
        log_config,
        TerminalMode::default(),
        ColorChoice::default(),
    )
    .unwrap();

    sound_manager_rust_lib::run();
}
