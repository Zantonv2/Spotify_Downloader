use tauri::State;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::config::{AppConfig, AudioQuality, AudioFormat};
use crate::search::{SearchQuery, SearchManager};
use crate::downloader::{manager::DownloadManager, DownloadStatus};
use crate::errors::AppError;
use crate::utils::sanitize_filename;
use crate::security::{SecureStorage, InputValidator};
// Plugin system removed
use crate::metadata::{lyrics::LyricsProvider, providers::MetadataProvider};

// State management
pub struct AppState {
    pub config: Arc<Mutex<AppConfig>>,
    pub search_manager: Arc<Mutex<SearchManager>>,
    pub download_manager: Arc<Mutex<DownloadManager>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub limit: Option<usize>,
    pub sources: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadRequest {
    pub track_id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub url: String,
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SettingsUpdate {
    pub download_path: Option<String>,
    pub max_concurrent_downloads: Option<usize>,
    pub preferred_quality: Option<String>,
    pub preferred_format: Option<String>,
    pub enable_metadata: Option<bool>,
    pub enable_lyrics: Option<bool>,
    pub enable_cover_art: Option<bool>,
    pub spotify_client_id: Option<String>,
    pub spotify_client_secret: Option<String>,
    pub musixmatch_client_id: Option<String>,
    pub musixmatch_client_secret: Option<String>,
    pub genius_client_id: Option<String>,
    pub genius_client_secret: Option<String>,
    pub deezer_api_key: Option<String>,
    // UI settings
    pub theme: Option<String>,
    pub show_notifications: Option<bool>,
    pub auto_start_downloads: Option<bool>,
    pub minimize_to_tray: Option<bool>,
    // Network settings
    pub proxy: Option<String>,
}

#[tauri::command]
pub async fn search_tracks(
    state: State<'_, AppState>,
    request: SearchRequest,
) -> std::result::Result<serde_json::Value, AppError> {
    let query = SearchQuery {
        query: request.query.clone(),
        limit: request.limit,
        platforms: request.sources,
        deep_search: false,
    };

    let search_manager = state.search_manager.lock().await;
    let result = search_manager.search(&query).await?;
    
    Ok(serde_json::to_value(result)?)
}

#[tauri::command]
pub async fn deep_search_tracks(
    state: State<'_, AppState>,
    query: String,
    limit: Option<usize>,
) -> std::result::Result<serde_json::Value, AppError> {
    let search_manager = state.search_manager.lock().await;
    let result = search_manager.deep_search(query, limit.unwrap_or(20)).await?;
    
    Ok(serde_json::to_value(result)?)
}

#[tauri::command]
pub async fn download_track(
    state: State<'_, AppState>,
    request: DownloadRequest,
) -> std::result::Result<String, AppError> {
    let config = state.config.lock().await;
    let output_dir = &config.download_path;
    
    // Create filename
    let filename = if let Some(album) = &request.album {
        format!("{} - {} - {}.{}", 
            sanitize_filename(&request.artist), 
            sanitize_filename(&request.title),
            sanitize_filename(album),
            config.get_format_extension()
        )
    } else {
        format!("{} - {}.{}", 
            sanitize_filename(&request.artist), 
            sanitize_filename(&request.title),
            config.get_format_extension()
        )
    };

    let tracks_dir = output_dir.join("tracks");
    let output_path = tracks_dir.join(filename);
    
    // Ensure output directory exists
    crate::utils::ensure_dir_exists(&tracks_dir).await?;

    println!("Adding download task for: {} - {}", request.artist, request.title);
    println!("URL: {}", request.url);
    println!("Output path: {:?}", output_path);

    let track_info = crate::api::TrackInfo {
        id: request.track_id,
        title: request.title,
        artist: request.artist,
        album: request.album,
        duration: None,
        year: None,
        genre: None,
        thumbnail_url: None,
        source: request.source,
        url: request.url,
        isrc: None,
        album_artist: None,
        track_number: None,
        disc_number: None,
        composer: None,
        quality: Some({
            let quality_str = match config.preferred_quality {
                AudioQuality::Low => "low".to_string(),
                AudioQuality::Medium => "medium".to_string(),
                AudioQuality::High => "high".to_string(),
                AudioQuality::Best => "best".to_string(),
            };
            println!("Sending quality to Python: '{}'", quality_str);
            quality_str
        }),
        format: Some(config.get_format_extension().to_string()),
    };

    let download_manager = state.download_manager.lock().await;
    
    // Get the next available order number for individual tracks
    let next_order = download_manager.get_next_individual_order().await;
    
    let task_id = match download_manager.add_download_with_order(track_info, output_path, false, next_order).await {
        Ok(id) => {
            println!("Successfully added download task with ID: {}", id);
            id
        },
        Err(e) => {
            println!("Failed to add download task: {}", e);
            return Err(e);
        }
    };
    
    Ok(task_id)
}

#[tauri::command]
pub async fn download_selected_tracks(
    state: State<'_, AppState>,
    task_ids: Vec<String>,
) -> std::result::Result<Vec<String>, AppError> {
    let download_manager = state.download_manager.lock().await;
    let mut started_tasks = Vec::new();
    
    for task_id in task_ids {
        match download_manager.start_download(task_id.clone()).await {
            Ok(_) => {
                println!("Successfully started download for task: {}", task_id);
                started_tasks.push(task_id);
            },
            Err(e) => {
                println!("Failed to start download for task {}: {}", task_id, e);
                // Continue with other tasks even if one fails
            }
        }
    }
    
    Ok(started_tasks)
}

#[tauri::command]
pub async fn get_download_queue(
    state: State<'_, AppState>,
) -> std::result::Result<Vec<crate::downloader::DownloadTask>, AppError> {
    let download_manager = state.download_manager.lock().await;
    let tasks = download_manager.get_all_tasks().await?;
    Ok(tasks)
}

#[tauri::command]
pub async fn pause_download(
    state: State<'_, AppState>,
    task_id: String,
) -> std::result::Result<(), AppError> {
    let download_manager = state.download_manager.lock().await;
    download_manager.pause_download(&task_id).await
}

#[tauri::command]
pub async fn resume_download(
    state: State<'_, AppState>,
    task_id: String,
) -> std::result::Result<(), AppError> {
    let download_manager = state.download_manager.lock().await;
    download_manager.resume_download(&task_id).await
}

#[tauri::command]
pub async fn remove_from_queue(
    state: State<'_, AppState>,
    task_id: String,
) -> std::result::Result<(), AppError> {
    let download_manager = state.download_manager.lock().await;
    download_manager.remove_download(&task_id).await
}

#[tauri::command]
pub async fn reorder_queue(
    _state: State<'_, AppState>,
    _task_ids: Vec<String>,
) -> std::result::Result<(), AppError> {
    // For now, this is a placeholder. In a full implementation,
    // we'd need to modify the download manager to support reordering
    Ok(())
}

// Additional Download Management Commands
#[tauri::command]
pub async fn download_all_pending(
    state: State<'_, AppState>,
) -> std::result::Result<serde_json::Value, AppError> {
    let download_manager = state.download_manager.lock().await;
    let tasks = download_manager.get_all_tasks().await?;
    
    let pending_tasks: Vec<String> = tasks
        .iter()
        .filter(|task| task.status == crate::downloader::DownloadStatus::Pending)
        .map(|task| task.id.clone())
        .collect();
    
    // Get the current concurrent download limit from config
    let config = state.config.lock().await;
    let max_concurrent = config.max_concurrent_downloads;
    drop(config); // Release the lock
    
    // Only start downloads up to the concurrent limit
    let mut started_count = 0;
    let mut failed_count = 0;
    
    for task_id in &pending_tasks {
        if started_count >= max_concurrent {
            log::info!("Reached concurrent download limit ({}), queuing remaining {} tasks", 
                max_concurrent, pending_tasks.len() - started_count);
            break;
        }
        
        match download_manager.start_download(task_id.clone()).await {
            Ok(_) => {
                started_count += 1;
                log::info!("Started download {} ({}/{})", task_id, started_count, max_concurrent);
            }
            Err(e) => {
                failed_count += 1;
                log::error!("Failed to start download {}: {}", task_id, e);
            }
        }
    }
    
    Ok(serde_json::json!({
        "success": true,
        "started_count": started_count,
        "failed_count": failed_count,
        "total_pending": pending_tasks.len(),
        "max_concurrent": max_concurrent,
        "queued_count": pending_tasks.len().saturating_sub(started_count)
    }))
}

#[tauri::command]
pub async fn download_selected(
    state: State<'_, AppState>,
    task_ids: Vec<String>,
) -> std::result::Result<serde_json::Value, AppError> {
    let download_manager = state.download_manager.lock().await;
    let mut started_count = 0;
    
    for task_id in &task_ids {
        if let Err(e) = download_manager.start_download(task_id.clone()).await {
            log::error!("Failed to start download {}: {}", task_id, e);
        } else {
            started_count += 1;
        }
    }
    
    Ok(serde_json::json!({
        "success": true,
        "started_count": started_count
    }))
}

#[tauri::command]
pub async fn pause_all_downloads(
    state: State<'_, AppState>,
) -> std::result::Result<serde_json::Value, AppError> {
    let download_manager = state.download_manager.lock().await;
    let tasks = download_manager.get_all_tasks().await?;
    
    let mut paused_count = 0;
    for task in tasks {
        if task.status == crate::downloader::DownloadStatus::Downloading {
            if let Err(e) = download_manager.pause_download(&task.id).await {
                log::error!("Failed to pause download {}: {}", task.id, e);
            } else {
                paused_count += 1;
            }
        }
    }
    
    Ok(serde_json::json!({
        "success": true,
        "paused_count": paused_count
    }))
}

#[tauri::command]
pub async fn resume_all_downloads(
    state: State<'_, AppState>,
) -> std::result::Result<serde_json::Value, AppError> {
    let download_manager = state.download_manager.lock().await;
    let tasks = download_manager.get_all_tasks().await?;
    
    let mut resumed_count = 0;
    for task in tasks {
        if task.status == crate::downloader::DownloadStatus::Paused {
            if let Err(e) = download_manager.resume_download(&task.id).await {
                log::error!("Failed to resume download {}: {}", task.id, e);
            } else {
                resumed_count += 1;
            }
        }
    }
    
    Ok(serde_json::json!({
        "success": true,
        "resumed_count": resumed_count
    }))
}

#[tauri::command]
pub async fn stop_all_downloads(
    state: State<'_, AppState>,
) -> std::result::Result<serde_json::Value, AppError> {
    let download_manager = state.download_manager.lock().await;
    let tasks = download_manager.get_all_tasks().await?;
    
    let mut stopped_count = 0;
    for task in tasks {
        if task.status == crate::downloader::DownloadStatus::Downloading || 
           task.status == crate::downloader::DownloadStatus::Paused {
            if let Err(e) = download_manager.cancel_download(&task.id).await {
                log::error!("Failed to stop download {}: {}", task.id, e);
            } else {
                stopped_count += 1;
            }
        }
    }
    
    Ok(serde_json::json!({
        "success": true,
        "stopped_count": stopped_count
    }))
}

#[tauri::command]
pub async fn clear_download_queue(
    state: State<'_, AppState>,
) -> std::result::Result<serde_json::Value, AppError> {
    let download_manager = state.download_manager.lock().await;
    let tasks = download_manager.get_all_tasks().await?;
    
    let mut cleared_count = 0;
    for task in tasks {
        if let Err(e) = download_manager.remove_download(&task.id).await {
            log::error!("Failed to remove download {}: {}", task.id, e);
        } else {
            cleared_count += 1;
        }
    }
    
    Ok(serde_json::json!({
        "success": true,
        "cleared_count": cleared_count
    }))
}

#[tauri::command]
pub async fn retry_download(
    state: State<'_, AppState>,
    task_id: String,
) -> std::result::Result<serde_json::Value, AppError> {
    let download_manager = state.download_manager.lock().await;
    
    // Reset the task status to pending and start it
    if let Some(mut task) = download_manager.get_task(&task_id).await? {
        task.status = crate::downloader::DownloadStatus::Pending;
        task.error = None;
        task.progress = 0.0;
        
        // Update the task in the manager
        download_manager.update_task_status(&task_id, crate::downloader::DownloadStatus::Pending, None).await?;
        
        // Start the download
        if let Err(e) = download_manager.start_download(task_id.clone()).await {
            log::error!("Failed to retry download {}: {}", task_id, e);
            return Err(AppError::DownloadError(format!("Failed to retry download: {}", e)));
        }
        
        Ok(serde_json::json!({
            "success": true,
            "task_id": task_id
        }))
    } else {
        Err(AppError::DownloadError("Task not found".to_string()))
    }
}

#[tauri::command]
pub async fn download_single(
    state: State<'_, AppState>,
    task_id: String,
) -> std::result::Result<serde_json::Value, AppError> {
    let download_manager = state.download_manager.lock().await;
    
    if let Err(e) = download_manager.start_download(task_id.clone()).await {
        log::error!("Failed to start download {}: {}", task_id, e);
        return Err(AppError::DownloadError(format!("Failed to start download: {}", e)));
    }
    
    Ok(serde_json::json!({
        "success": true,
        "task_id": task_id
    }))
}

#[tauri::command]
pub async fn process_download_queue(
    state: State<'_, AppState>,
) -> std::result::Result<serde_json::Value, AppError> {
    let download_manager = state.download_manager.lock().await;
    
    if let Err(e) = download_manager.process_queue().await {
        log::error!("Failed to process download queue: {}", e);
        return Err(AppError::DownloadError(format!("Failed to process queue: {}", e)));
    }
    
    Ok(serde_json::json!({
        "success": true,
        "message": "Queue processing initiated"
    }))
}

#[tauri::command]
pub async fn get_settings(
    state: State<'_, AppState>,
) -> std::result::Result<serde_json::Value, AppError> {
    let config = state.config.lock().await;
    
    // Create a frontend-compatible config object
    let frontend_config = serde_json::json!({
        "download_path": config.download_path.to_string_lossy().to_string(),
        "max_concurrent_downloads": config.max_concurrent_downloads,
        "preferred_quality": match config.preferred_quality {
            AudioQuality::Low => "low",
            AudioQuality::Medium => "medium", 
            AudioQuality::High => "high",
            AudioQuality::Best => "best",
        },
        "preferred_format": match config.preferred_format {
            AudioFormat::Mp3 => "mp3",
            AudioFormat::M4a => "m4a",
            AudioFormat::Flac => "flac",
            AudioFormat::Wav => "wav",
        },
        "enable_metadata": config.enable_metadata,
        "enable_lyrics": config.enable_lyrics,
        "enable_cover_art": config.enable_cover_art,
        "api_keys": {
            "spotify_client_id": config.api_keys.spotify_client_id,
            "spotify_client_secret": config.api_keys.spotify_client_secret,
            "musicbrainz_user_agent": config.api_keys.musicbrainz_user_agent,
            "musixmatch_client_id": config.api_keys.musixmatch_client_id,
            "musixmatch_client_secret": config.api_keys.musixmatch_client_secret,
            "genius_client_id": config.api_keys.genius_client_id,
            "genius_client_secret": config.api_keys.genius_client_secret,
            "deezer_api_key": config.api_keys.deezer_api_key,
        },
        "ui": {
            "theme": config.ui.theme,
            "show_notifications": config.ui.show_notifications,
            "auto_start_downloads": config.ui.auto_start_downloads,
            "minimize_to_tray": config.ui.minimize_to_tray,
        },
        "proxy": config.proxy
    });
    
    Ok(frontend_config)
}

#[tauri::command]
pub async fn update_settings(
    state: State<'_, AppState>,
    update: SettingsUpdate,
) -> std::result::Result<(), AppError> {
    let mut config = state.config.lock().await;
    
    if let Some(path) = update.download_path {
        config.download_path = std::path::PathBuf::from(path);
    }
    
    if let Some(max_downloads) = update.max_concurrent_downloads {
        config.max_concurrent_downloads = max_downloads;
    }
    
    if let Some(quality) = update.preferred_quality {
        config.preferred_quality = match quality.as_str() {
            "low" => AudioQuality::Low,
            "medium" => AudioQuality::Medium,
            "high" => AudioQuality::High,
            "best" => AudioQuality::Best,
            _ => AudioQuality::High,
        };
    }
    
    if let Some(format) = update.preferred_format {
        config.preferred_format = match format.as_str() {
            "mp3" => AudioFormat::Mp3,
            "m4a" => AudioFormat::M4a,
            "flac" => AudioFormat::Flac,
            "wav" => AudioFormat::Wav,
            _ => AudioFormat::Mp3,
        };
    }
    
    if let Some(enable) = update.enable_metadata {
        config.enable_metadata = enable;
    }
    
    if let Some(enable) = update.enable_lyrics {
        config.enable_lyrics = enable;
    }
    
    if let Some(enable) = update.enable_cover_art {
        config.enable_cover_art = enable;
    }
    
    if let Some(client_id) = update.spotify_client_id {
        config.api_keys.spotify_client_id = Some(client_id);
    }
    
    if let Some(client_secret) = update.spotify_client_secret {
        config.api_keys.spotify_client_secret = Some(client_secret);
    }
    
    if let Some(client_id) = update.musixmatch_client_id {
        config.api_keys.musixmatch_client_id = Some(client_id);
    }
    
    if let Some(client_secret) = update.musixmatch_client_secret {
        config.api_keys.musixmatch_client_secret = Some(client_secret);
    }
    
    if let Some(client_id) = update.genius_client_id {
        config.api_keys.genius_client_id = Some(client_id);
    }
    
    if let Some(client_secret) = update.genius_client_secret {
        config.api_keys.genius_client_secret = Some(client_secret);
    }
    
    if let Some(api_key) = update.deezer_api_key {
        config.api_keys.deezer_api_key = Some(api_key);
    }
    
    // Handle UI settings
    if let Some(theme) = update.theme {
        config.ui.theme = theme;
    }
    
    if let Some(show_notifications) = update.show_notifications {
        config.ui.show_notifications = show_notifications;
    }
    
    if let Some(auto_start_downloads) = update.auto_start_downloads {
        config.ui.auto_start_downloads = auto_start_downloads;
    }
    
    if let Some(minimize_to_tray) = update.minimize_to_tray {
        config.ui.minimize_to_tray = minimize_to_tray;
    }
    
    // Handle network settings
    if let Some(proxy) = update.proxy {
        config.proxy = Some(proxy);
    }
    
    config.save()?;
    Ok(())
}

#[tauri::command]
pub async fn get_metadata_sources(
    state: State<'_, AppState>,
) -> std::result::Result<serde_json::Value, AppError> {
    let search_manager = state.search_manager.lock().await;
    let sources = search_manager.get_available_sources();
    Ok(serde_json::to_value(sources)?)
}

#[tauri::command]
pub async fn validate_api_key(
    _state: State<'_, AppState>,
    _service: String,
    _api_key: String,
) -> std::result::Result<bool, AppError> {
    // This would validate the API key for the specified service
    // For now, just return true as a placeholder
    Ok(true)
}

#[tauri::command]
pub async fn get_download_progress(
    state: State<'_, AppState>,
    task_id: String,
) -> std::result::Result<serde_json::Value, AppError> {
    let download_manager = state.download_manager.lock().await;
    let progress = download_manager.get_progress(&task_id).await?;
    Ok(serde_json::to_value(progress)?)
}

#[tauri::command]
pub async fn get_app_stats(
    state: State<'_, AppState>,
) -> std::result::Result<serde_json::Value, AppError> {
    let download_manager = state.download_manager.lock().await;
    let tasks = download_manager.get_all_tasks().await?;
    let total_tasks = tasks.len();
    let completed_tasks = tasks.iter().filter(|t| t.status == DownloadStatus::Completed).count();
    let failed_tasks = tasks.iter().filter(|t| t.status == DownloadStatus::Failed).count();
    let downloading_tasks = tasks.iter().filter(|t| t.status == DownloadStatus::Downloading).count();
    
    let stats = serde_json::json!({
        "total_tasks": total_tasks,
        "completed_tasks": completed_tasks,
        "failed_tasks": failed_tasks,
        "downloading_tasks": downloading_tasks,
        "success_rate": if total_tasks > 0 { (completed_tasks as f64 / total_tasks as f64) * 100.0 } else { 0.0 }
    });
    
    Ok(stats)
}

#[tauri::command]
pub async fn browse_folder(
    app: tauri::AppHandle,
) -> std::result::Result<Option<String>, AppError> {
    use tauri_plugin_dialog::DialogExt;
    use std::sync::mpsc;
    
    let (tx, rx) = mpsc::channel();
    
    app.dialog().file()
        .set_title("Select Download Folder")
        .pick_folder(move |path| {
            let _ = tx.send(path);
        });
    
    match rx.recv() {
        Ok(Some(path)) => Ok(Some(path.to_string())),
        Ok(None) => Ok(None),
        Err(_) => Ok(None),
    }
}

#[tauri::command]
pub async fn store_api_key(
    service: String,
    api_key: String,
) -> std::result::Result<(), AppError> {
    let validator = InputValidator::new();
    validator.validate_api_key(&service, &api_key)?;
    
    let storage = SecureStorage::new()?;
    storage.store_api_key(&service, &api_key).await?;
    
    Ok(())
}

#[tauri::command]
pub async fn get_api_key(
    service: String,
) -> std::result::Result<Option<String>, AppError> {
    let storage = SecureStorage::new()?;
    storage.get_api_key(&service).await
}

#[tauri::command]
pub async fn remove_api_key(
    service: String,
) -> std::result::Result<(), AppError> {
    let storage = SecureStorage::new()?;
    storage.remove_api_key(&service).await?;
    Ok(())
}

#[tauri::command]
pub async fn list_stored_services() -> std::result::Result<Vec<String>, AppError> {
    let storage = SecureStorage::new()?;
    storage.list_services().await
}

#[tauri::command]
pub async fn validate_input(
    input_type: String,
    value: String,
) -> std::result::Result<(), AppError> {
    let validator = InputValidator::new();
    
    match input_type.as_str() {
        "url" => validator.validate_url(&value),
        "file_path" => validator.validate_file_path(&value),
        "search_query" => validator.validate_search_query(&value),
        "download_path" => validator.validate_download_path(&value),
        _ => Err(AppError::Validation(format!("Unknown input type: {}", input_type))),
    }
}


// Plugin functions removed

#[tauri::command]
pub async fn search_lyrics(
    artist: String,
    title: String,
) -> std::result::Result<Option<String>, AppError> {
    let lyrics_provider = LyricsProvider::new();
    lyrics_provider.search_lyrics(&artist, &title).await
}

// Metadata embedding functions removed - using Python for metadata embedding

#[tauri::command]
pub async fn search_enhanced_metadata(
    artist: String,
    title: String,
) -> std::result::Result<Option<crate::metadata::MetadataInfo>, AppError> {
    let metadata_provider = MetadataProvider::new();
    // TODO: Load API keys from secure storage
    metadata_provider.search_metadata(&artist, &title).await
}

#[tauri::command]
pub async fn search_cover_art(
    artist: String,
    title: String,
    album: Option<String>,
) -> std::result::Result<Option<crate::metadata::CoverArtInfo>, AppError> {
    let metadata_provider = MetadataProvider::new();
    // TODO: Load API keys from secure storage
    metadata_provider.search_cover_art(&artist, &title, album.as_deref()).await
}

#[tauri::command]
pub async fn set_lyrics_api_key(
    service: String,
    api_key: String,
) -> std::result::Result<(), AppError> {
    // Store API key securely
    let storage = SecureStorage::new()?;
    storage.store_api_key(&service, &api_key).await?;
    Ok(())
}

#[tauri::command]
pub async fn set_metadata_api_key(
    service: String,
    api_key: String,
) -> std::result::Result<(), AppError> {
    // Store API key securely
    let storage = SecureStorage::new()?;
    storage.store_api_key(&service, &api_key).await?;
    Ok(())
}

#[tauri::command]
pub async fn set_proxy_url(
    proxy_url: Option<String>,
) -> std::result::Result<(), AppError> {
    if let Some(url) = &proxy_url {
        // Validate proxy URL format
        if !url.starts_with("http://") && !url.starts_with("https://") && !url.starts_with("socks5://") {
            return Err(AppError::InvalidInput("Proxy URL must start with http://, https://, or socks5://".to_string()));
        }
        
        // Set environment variables for the current process
        std::env::set_var("HTTP_PROXY", url);
        std::env::set_var("HTTPS_PROXY", url);
        std::env::set_var("ALL_PROXY", url);
        
        log::info!("Proxy configured: {}", url);
    } else {
        // Remove proxy settings
        std::env::remove_var("HTTP_PROXY");
        std::env::remove_var("HTTPS_PROXY");
        std::env::remove_var("ALL_PROXY");
        
        log::info!("Proxy settings cleared");
    }
    
    Ok(())
}

#[tauri::command]
pub async fn get_proxy_url() -> std::result::Result<Option<String>, AppError> {
    Ok(std::env::var("HTTP_PROXY")
        .or_else(|_| std::env::var("HTTPS_PROXY"))
        .or_else(|_| std::env::var("ALL_PROXY"))
        .ok())
}

#[tauri::command]
pub async fn get_available_lyrics_services() -> std::result::Result<Vec<String>, AppError> {
    Ok(vec![
        "lrclib".to_string(),
        "lyrics.ovh".to_string(),
        "musixmatch".to_string(),
        "genius".to_string(),
    ])
}

#[tauri::command]
pub async fn get_available_metadata_services() -> std::result::Result<Vec<String>, AppError> {
    Ok(vec![
        "spotify".to_string(),
        "musicbrainz".to_string(),
        "deezer".to_string(),
    ])
}

#[tauri::command]
pub async fn get_available_cover_art_services() -> std::result::Result<Vec<String>, AppError> {
    Ok(vec![
        "spotify".to_string(),
        "itunes".to_string(),
    ])
}

#[tauri::command]
pub async fn check_ffmpeg_availability() -> std::result::Result<bool, AppError> {
    use std::process::Command;
    
    // Get the FFmpeg path using the same logic as get_ffmpeg_path
    let ffmpeg_path = get_ffmpeg_path().await?;
    
    if let Some(path) = ffmpeg_path {
        // Test if the found FFmpeg actually works
        let ffmpeg_result = Command::new(&path)
            .arg("-version")
            .output();
        
        // Also check for ffprobe in the same directory
        let ffprobe_path = path.replace("ffmpeg.exe", "ffprobe.exe");
        let ffprobe_result = Command::new(&ffprobe_path)
            .arg("-version")
            .output();
        
        Ok(ffmpeg_result.is_ok() && ffprobe_result.is_ok())
    } else {
        Ok(false)
    }
}

#[tauri::command]
pub async fn get_ffmpeg_path() -> std::result::Result<Option<String>, AppError> {
    use std::process::Command;
    
    // Try to find ffmpeg in PATH
    if let Ok(output) = Command::new("where").arg("ffmpeg").output() {
        if let Ok(path) = String::from_utf8(output.stdout) {
            let path = path.trim().lines().next().unwrap_or("").to_string();
            if !path.is_empty() {
                return Ok(Some(path));
            }
        }
    }
    
    // Try common installation paths
    let common_paths = vec![
        "C:\\Users\\temaz\\Downloads\\ffmpeg-master-latest-win64-gpl-shared\\bin\\ffmpeg.exe", // Hardcoded primary path
        "C:\\ffmpeg\\bin\\ffmpeg.exe",
        "C:\\Program Files\\ffmpeg\\bin\\ffmpeg.exe",
        "C:\\Program Files (x86)\\ffmpeg\\bin\\ffmpeg.exe",
        "C:\\Program Files\\Jellyfin\\Server\\ffmpeg.exe",
    ];
    
    for path in common_paths {
        if std::path::Path::new(path).exists() {
            return Ok(Some(path.to_string()));
        }
    }
    
    Ok(None)
}

// Spotify Import Commands
#[tauri::command]
pub async fn import_spotify_url(
    _app: tauri::AppHandle,
    state: State<'_, AppState>,
    url: String,
    client_id: String,
    client_secret: String,
) -> std::result::Result<serde_json::Value, AppError> {
    log::info!("Importing Spotify URL: {}", url);
    
    // Parse Spotify URL to determine type and extract ID
    let (_url_type, _spotify_id) = parse_spotify_url(&url)?;
    
    // Get proxy setting from config
    let config = state.config.lock().await;
    let proxy = config.proxy.clone();
    drop(config);
    
    // Use relative path from src-tauri directory to project root
    let script_path = "../python_processor/spotify_client.py";
    
    // Use Python Spotipy client for full playlist/album support
    let result = crate::utils::execute_python_script_with_ffmpeg(
        script_path,
        serde_json::json!({
            "client_id": client_id,
            "client_secret": client_secret,
            "url": url,
            "proxy": proxy
        }),
        None
    ).await?;
    
    Ok(result)
}

#[tauri::command]
pub async fn import_csv_playlist(
    _state: State<'_, AppState>,
    file_path: String,
) -> std::result::Result<serde_json::Value, AppError> {
    log::info!("Importing CSV playlist from: {}", file_path);
    
    let csv_data = read_csv_playlist(&file_path).await?;
    
    Ok(serde_json::json!({
        "success": true,
        "tracks": csv_data,
        "total": csv_data.len()
    }))
}

#[tauri::command]
pub async fn bulk_download_spotify_tracks(
    state: State<'_, AppState>,
    tracks: Vec<serde_json::Value>,
) -> std::result::Result<serde_json::Value, AppError> {
    log::info!("Starting bulk download of {} tracks", tracks.len());
    
    // Get the configured download path and settings
    let (output_dir, preferred_quality, preferred_format) = {
        let config = state.config.lock().await;
        (
            config.download_path.clone(),
            config.preferred_quality.clone(),
            config.preferred_format.clone()
        )
    };
    
    let download_manager = state.download_manager.lock().await;
    let mut download_ids = Vec::new();
    
    for (index, track) in tracks.iter().enumerate() {
        log::info!("Processing track {}: {:?}", index + 1, track);
        
        if let Some(mut track_info) = parse_spotify_track(track.clone()) {
            // Add track number to metadata
            track_info.track_number = Some((index + 1) as u32);
            
            // Set quality and format from user preferences
            track_info.quality = Some(match preferred_quality {
                AudioQuality::Low => "low".to_string(),
                AudioQuality::Medium => "medium".to_string(),
                AudioQuality::High => "high".to_string(),
                AudioQuality::Best => "best".to_string(),
            });
            track_info.format = Some(match preferred_format {
                AudioFormat::Mp3 => "mp3".to_string(),
                AudioFormat::M4a => "m4a".to_string(),
                AudioFormat::Flac => "flac".to_string(),
                AudioFormat::Wav => "wav".to_string(),
            });
            
            // Use sanitized filename with proper path and correct extension
            let sanitized_filename = crate::utils::sanitize_track_filename(&track_info.artist, &track_info.title);
            let extension = match preferred_format {
                AudioFormat::Mp3 => "mp3",
                AudioFormat::M4a => "m4a",
                AudioFormat::Flac => "flac",
                AudioFormat::Wav => "wav",
            };
            let tracks_dir = output_dir.join("tracks");
            let output_path = tracks_dir.join(format!("{}.{}", sanitized_filename, extension));
            
            log::info!("Adding download: {} - {} -> {:?}", track_info.artist, track_info.title, output_path);
            
            // Use order-based download method with proper track order
            match download_manager.add_download_with_order(track_info, output_path, false, (index + 1) as u32).await {
                Ok(task_id) => {
                    log::info!("Successfully added download with ID: {} (order: {})", task_id, index + 1);
                    download_ids.push(task_id);
                },
                Err(e) => {
                    log::error!("Failed to add download: {}", e);
                }
            }
        } else {
            log::error!("Failed to parse track: {:?}", track);
        }
    }
    
    log::info!("Bulk download setup complete. Added {} tracks to queue", download_ids.len());
    
    Ok(serde_json::json!({
        "success": true,
        "download_ids": download_ids,
        "total": download_ids.len()
    }))
}

// Helper functions
#[derive(Debug)]
enum SpotifyUrlType {
    Track,
    Album,
    Playlist,
}

fn parse_spotify_url(url: &str) -> Result<(SpotifyUrlType, String), AppError> {
    // Parse Spotify URLs like:
    // https://open.spotify.com/track/4iV5W9uYEdYUVa79Axb7Rh
    // https://open.spotify.com/album/1A2GTWGtFfWp7KSQTwWOyo
    // https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M
    
    if let Some(captures) = regex::Regex::new(r"https://open\.spotify\.com/(track|album|playlist)/([a-zA-Z0-9]+)")
        .unwrap()
        .captures(url) {
        
        let url_type = match &captures[1] {
            "track" => SpotifyUrlType::Track,
            "album" => SpotifyUrlType::Album,
            "playlist" => SpotifyUrlType::Playlist,
            _ => return Err(AppError::InvalidInput("Invalid Spotify URL type".to_string())),
        };
        
        let spotify_id = captures[2].to_string();
        Ok((url_type, spotify_id))
    } else {
        Err(AppError::InvalidInput("Invalid Spotify URL format".to_string()))
    }
}

async fn read_csv_playlist(file_path: &str) -> Result<Vec<serde_json::Value>, AppError> {
    use tokio::fs;
    
    let content = fs::read_to_string(file_path).await
        .map_err(|e| AppError::IoError(format!("Failed to read CSV file: {}", e)))?;
    
    let mut tracks = Vec::new();
    let mut csv_reader = csv::Reader::from_reader(content.as_bytes());
    
    let mut track_number = 1;
    for result in csv_reader.records() {
        match result {
            Ok(record) => {
                // Skip header row
                if record.get(0).unwrap_or("") == "Position" {
                    continue;
                }
                
                let track = parse_csv_track_record(&record, track_number)?;
                tracks.push(track);
                track_number += 1;
            }
            Err(e) => {
                log::warn!("Failed to parse CSV record: {}", e);
                continue;
            }
        }
    }
    
    Ok(tracks)
}

fn parse_csv_track_record(record: &csv::StringRecord, track_number: u32) -> Result<serde_json::Value, AppError> {
    let track_name = record.get(1).unwrap_or("Unknown").to_string();
    let album_name = record.get(2).unwrap_or("Unknown").to_string();
    let artist_name = record.get(3).unwrap_or("Unknown").to_string()
        .replace(";", ", "); // Convert semicolons to commas with spaces
    let release_date = record.get(4).unwrap_or("").to_string();
    let duration_ms = record.get(5).unwrap_or("0").parse::<u64>().unwrap_or(0);
    let popularity = record.get(6).unwrap_or("0").parse::<u32>().unwrap_or(0);
    let genres = record.get(10).unwrap_or("").to_string();
    
    Ok(serde_json::json!({
        "title": track_name,
        "artist": artist_name,
        "album": album_name,
        "year": extract_year_from_date(&release_date),
        "duration": duration_ms / 1000, // Convert to seconds
        "popularity": popularity,
        "genres": genres,
        "track_number": track_number,
        "source": "spotify_csv"
    }))
}

fn parse_spotify_track(track_data: serde_json::Value) -> Option<crate::api::TrackInfo> {
    let title = track_data.get("title")?.as_str()?.to_string();
    let artist = track_data.get("artist")?.as_str()?.to_string();
    
    // Handle different data sources
    let (id, source) = if let Some(id) = track_data.get("id").and_then(|v| v.as_str()) {
        // Spotify API data
        (id.to_string(), "spotify".to_string())
    } else {
        // CSV data - generate a unique ID
        let id = format!("csv_{}_{}", 
            artist.replace(" ", "_").to_lowercase(), 
            title.replace(" ", "_").to_lowercase()
        );
        (id, "spotify_csv".to_string())
    };
    
    // Get thumbnail URL (try multiple possible fields)
    let thumbnail_url = track_data.get("thumbnail_url")
        .or_else(|| track_data.get("external_urls").and_then(|urls| urls.get("spotify")))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    
    Some(crate::api::TrackInfo {
        id,
        title,
        artist,
        album: track_data.get("album").and_then(|v| v.as_str()).map(|s| s.to_string()),
        duration: track_data.get("duration").and_then(|v| v.as_u64()),
        thumbnail_url,
        source,
        url: String::new(), // Empty URL - will trigger search instead of direct download
        quality: None,
        format: None,
        year: track_data.get("year").and_then(|v| v.as_u64()).map(|n| n as u32),
        genre: track_data.get("genres").and_then(|v| v.as_str()).map(|s| s.to_string()),
        isrc: None,
        album_artist: None,
        track_number: track_data.get("track_number").and_then(|v| v.as_u64()).map(|n| n as u32),
        disc_number: track_data.get("disc_number").and_then(|v| v.as_u64()).map(|n| n as u32),
        composer: None,
    })
}

fn extract_year_from_date(date_str: &str) -> Option<u32> {
    // Extract year from date strings like "2014-10-06" or "2019"
    if date_str.len() >= 4 {
        date_str[0..4].parse::<u32>().ok()
    } else {
        None
    }
}
