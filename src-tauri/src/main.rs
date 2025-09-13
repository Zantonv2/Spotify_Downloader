// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod config;
mod errors;
mod utils;
mod api;
mod search;
mod downloader;
mod metadata;
mod processing;
mod security;
#[cfg(test)]
mod search_test;

use log::info;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    // Initialize logger
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    info!("Starting Spotify Downloader application");

    // Load configuration
    let config = match config::AppConfig::load() {
        Ok(config) => {
            info!("Configuration loaded successfully");
            config
        }
        Err(e) => {
            log::error!("Failed to load configuration: {}", e);
            config::AppConfig::default()
        }
    };

    // Initialize proxy settings from config
    if let Some(proxy_url) = &config.proxy {
        info!("Setting up proxy: {}", proxy_url);
        std::env::set_var("HTTP_PROXY", proxy_url);
        std::env::set_var("HTTPS_PROXY", proxy_url);
        std::env::set_var("ALL_PROXY", proxy_url);
    }

    // Initialize search manager
    let search_manager = Arc::new(Mutex::new(search::SearchManager::new()));
    info!("Search manager initialized successfully");

    // Initialize download manager
    let download_manager = Arc::new(Mutex::new(downloader::manager::DownloadManager::new(
        config.max_concurrent_downloads
    )));

    // Create app state
    let app_state = commands::AppState {
        config: Arc::new(Mutex::new(config)),
        search_manager,
        download_manager,
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::search_tracks,
            commands::deep_search_tracks,
            commands::download_track,
            commands::get_download_queue,
            commands::pause_download,
            commands::resume_download,
            commands::remove_from_queue,
            commands::reorder_queue,
            commands::get_settings,
            commands::update_settings,
            commands::get_metadata_sources,
            commands::validate_api_key,
            commands::get_download_progress,
            commands::get_app_stats,
            commands::browse_folder,
            commands::store_api_key,
            commands::get_api_key,
            commands::remove_api_key,
            commands::list_stored_services,
            commands::validate_input,
            // Plugin commands removed
            commands::search_lyrics,
            // Metadata embedding commands removed - using Python
            commands::search_enhanced_metadata,
            commands::search_cover_art,
            commands::set_lyrics_api_key,
            commands::set_metadata_api_key,
            commands::set_proxy_url,
            commands::get_proxy_url,
            commands::get_available_lyrics_services,
            commands::get_available_metadata_services,
            commands::get_available_cover_art_services,
            commands::check_ffmpeg_availability,
            commands::get_ffmpeg_path,
            commands::import_spotify_url,
            commands::import_csv_playlist,
            commands::bulk_download_spotify_tracks,
            commands::download_all_pending,
            commands::download_selected,
            commands::pause_all_downloads,
            commands::resume_all_downloads,
            commands::stop_all_downloads,
            commands::clear_download_queue,
            commands::retry_download,
            commands::download_single
        ])
        .setup(|app| {
            info!("Application setup completed");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
