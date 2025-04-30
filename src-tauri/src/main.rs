// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(debug_assertions)]
    let log_level = log::LevelFilter::Debug;

    #[cfg(not(debug_assertions))]
    let log_level = log::LevelFilter::Info;

    let log_config = simplelog::ConfigBuilder::new()
        .add_filter_allow_str("sound_manager_rust_lib")
        .build();

    simplelog::TermLogger::init(
        log_level,
        log_config,
        Default::default(),
        Default::default(),
    )
    .unwrap();

    sound_manager_rust_lib::run()
}
