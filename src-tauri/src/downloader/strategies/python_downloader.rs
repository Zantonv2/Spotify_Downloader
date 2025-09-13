use super::super::super::downloader::{Downloader, DownloadTask, DownloadProgress};
use super::super::super::errors::Result;
use super::super::super::utils::execute_python_script_with_ffmpeg;
use super::super::super::commands::get_ffmpeg_path;
use serde_json::json;
use regex;
use async_trait::async_trait;

pub struct PythonDownloader {
    name: String,
}

impl PythonDownloader {
    pub fn new() -> Self {
        Self {
            name: "python-downloader".to_string(),
        }
    }
}

#[async_trait]
impl Downloader for PythonDownloader {
    async fn download(&self, task: &DownloadTask) -> Result<()> {
        log::info!("Started Downloading: {} - {}", task.track_info.artist, task.track_info.title);
        
        // Get FFmpeg path
        let ffmpeg_path = get_ffmpeg_path().await?;
        
        // Get download directory from output path (parent of tracks folder)
        let download_dir = task.output_path.parent()
            .and_then(|p| p.parent()) // Go up one more level from tracks folder
            .unwrap_or(std::path::Path::new("."))
            .to_string_lossy()
            .to_string();

        // Step 1: Download audio file (without metadata embedding)
        let download_request = json!({
            "action": "download",
            "task_id": task.id,
            "url": task.track_info.url,
            "output_path": task.output_path.to_string_lossy(),
            "download_dir": download_dir,
            "format": task.track_info.format.as_ref().unwrap_or(&"mp3".to_string()),
            "quality": task.track_info.quality.as_ref().unwrap_or(&"high".to_string()),
            "title": task.track_info.title,
            "artist": task.track_info.artist,
            "album": task.track_info.album,
            "year": task.track_info.year,
            "genre": task.track_info.genre,
            "thumbnail_url": task.track_info.thumbnail_url
        });

        // Execute Python download script
        let script_path = "../python_processor/audio_processor.py";
        let result = execute_python_script_with_ffmpeg(script_path, download_request, ffmpeg_path.clone()).await?;

        log::info!("Python script response: {:?}", result);

        // Check if download was successful
        if let Some(success) = result.get("success").and_then(|v| v.as_bool()) {
            if !success {
                let error_msg = result.get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown download error");
                log::error!("Python download failed: {}", error_msg);
                return Err(crate::errors::AppError::DownloadError(format!("Download failed: {}", error_msg)));
            }
        } else {
            log::error!("Invalid response from Python download script - missing 'success' field");
            log::error!("Response keys: {:?}", result.as_object().map(|obj| obj.keys().collect::<Vec<_>>()));
            return Err(crate::errors::AppError::DownloadError("Invalid response from download script".to_string()));
        }

        log::info!("Downloaded Track: {} - {}", task.track_info.artist, task.track_info.title);

        // Step 2: Search for enhanced metadata and lyrics in parallel for much faster results
        log::info!("Searching for metadata and lyrics...");
        
        // Parse track info for lyrics search
        let (clean_artist, clean_title) = self.parse_track_info(&task.track_info.artist, &task.track_info.title);
        log::info!("🎵 Parsed track info - Artist: '{}', Title: '{}'", clean_artist, clean_title);
        
        // Run metadata and lyrics search in parallel
        let metadata_future = self.search_enhanced_metadata(&task.track_info.artist, &task.track_info.title);
        let proxy_url = self.get_proxy_from_config().await;
        let lyrics_provider = crate::metadata::lyrics::LyricsProvider::new_with_proxy(proxy_url);
        let lyrics_future = lyrics_provider.search_lyrics(&clean_artist, &clean_title);
        
        // Wait for both to complete
        let (metadata_result, lyrics_result) = tokio::join!(metadata_future, lyrics_future);
        
        let enhanced_metadata = metadata_result?;
        
        // Step 3: Embed enhanced metadata using Python
        if let Some(mut metadata) = enhanced_metadata {
            log::info!("Found metadata");
            
            // Add lyrics if found
            match lyrics_result {
                Ok(Some(lyrics)) => {
                    log::info!("Found lyrics");
                    metadata.lyrics = Some(lyrics);
                }
                Ok(None) => {
                    log::info!("No lyrics found");
                }
                Err(e) => {
                    log::warn!("Lyrics search failed: {}", e);
                }
            }
            
            let final_metadata = metadata;
            
            
            // Sanitize Unicode strings to prevent JSON serialization errors
            let sanitize_string = |s: Option<String>| -> Option<String> {
                s.map(|s| {
                    // Remove or replace problematic Unicode characters
                    s.chars()
                        .filter(|c| c.is_ascii() || !c.is_control())
                        .collect::<String>()
                        .trim()
                        .to_string()
                }).filter(|s| !s.is_empty())
            };

            // Create a safe metadata structure with lyrics
            // Use queue position as track number instead of original album track number
            let safe_metadata = serde_json::json!({
                "title": sanitize_string(Some(final_metadata.title)),
                "artist": sanitize_string(Some(final_metadata.artist)),
                "album": sanitize_string(final_metadata.album),
                "year": final_metadata.year,
                "genre": sanitize_string(final_metadata.genre),
                "track_number": task.order, // Use queue position as track number
                "disc_number": final_metadata.disc_number,
                "album_artist": sanitize_string(final_metadata.album_artist),
                "composer": sanitize_string(final_metadata.composer),
                "isrc": sanitize_string(final_metadata.isrc),
                "cover_art_url": sanitize_string(final_metadata.cover_art_url.clone()),
                "lyrics": sanitize_string(final_metadata.lyrics.clone())
            });

            // Embed basic metadata using Python
            let embed_request = json!({
                "action": "embed_metadata",
                "file_path": task.output_path.to_string_lossy(),
                "metadata": safe_metadata
            });

            let embed_result = execute_python_script_with_ffmpeg(script_path, embed_request, ffmpeg_path.clone()).await?;
            
            if let Some(success) = embed_result.get("success").and_then(|v| v.as_bool()) {
                if success {
                    log::info!("Basic metadata embedded successfully");
                } else {
                    let error_msg = embed_result.get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown embedding error");
                    log::warn!("Basic metadata embedding failed: {}", error_msg);
                }
            } else {
                log::warn!("Invalid response from basic metadata embedding script");
            }

            // Embed cover art if available
            if let Some(cover_art_url) = &final_metadata.cover_art_url {
                if !cover_art_url.is_empty() {
                    let cover_request = json!({
                        "action": "embed_cover_art",
                        "file_path": task.output_path.to_string_lossy(),
                        "cover_art": {
                            "url": cover_art_url
                        }
                    });

                    let cover_result = execute_python_script_with_ffmpeg(script_path, cover_request, ffmpeg_path.clone()).await?;
                    
                    if let Some(success) = cover_result.get("success").and_then(|v| v.as_bool()) {
                        if success {
                            log::info!("Cover art embedded successfully");
                        } else {
                            let error_msg = cover_result.get("error")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Unknown cover art error");
                            log::warn!("Cover art embedding failed: {}", error_msg);
                        }
                    }
                }
            }

            // Embed lyrics if available
            if let Some(lyrics) = &final_metadata.lyrics {
                if !lyrics.is_empty() {
                    let lyrics_request = json!({
                        "action": "embed_lyrics",
                        "file_path": task.output_path.to_string_lossy(),
                        "lyrics": lyrics
                    });

                    let lyrics_result = execute_python_script_with_ffmpeg(script_path, lyrics_request, ffmpeg_path.clone()).await?;
                    
                    if let Some(success) = lyrics_result.get("success").and_then(|v| v.as_bool()) {
                        if success {
                            log::info!("Lyrics embedded successfully");
                        } else {
                            let error_msg = lyrics_result.get("error")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Unknown lyrics error");
                            log::warn!("Lyrics embedding failed: {}", error_msg);
                        }
                    }
                }
            }
        } else {
            log::info!("No enhanced metadata found, using basic metadata");
        }

        log::info!("Completed Download: {} - {}", task.track_info.artist, task.track_info.title);
        Ok(())
    }

    async fn pause(&self, _task_id: &str) -> Result<()> {
        // Python downloader doesn't support pausing
        Err(crate::errors::AppError::DownloadError("Pause not supported".to_string()))
    }

    async fn resume(&self, _task_id: &str) -> Result<()> {
        // Python downloader doesn't support resuming
        Err(crate::errors::AppError::DownloadError("Resume not supported".to_string()))
    }

    async fn cancel(&self, _task_id: &str) -> Result<()> {
        // Python downloader doesn't support cancellation
        Err(crate::errors::AppError::DownloadError("Cancel not supported".to_string()))
    }

    async fn get_progress(&self, _task_id: &str) -> Result<DownloadProgress> {
        // Python downloader doesn't support progress tracking
        Err(crate::errors::AppError::DownloadError("Progress tracking not supported".to_string()))
    }

    fn supports_format(&self, format: &str) -> bool {
        matches!(format, "mp3" | "m4a" | "flac" | "wav")
    }

    fn get_name(&self) -> &str {
        &self.name
    }
}

impl PythonDownloader {
    async fn search_enhanced_metadata(&self, artist: &str, title: &str) -> Result<Option<crate::metadata::MetadataInfo>> {
        // Parse and clean the track information
        let (clean_artist, clean_title) = self.parse_track_info(artist, title);
        
        // Get proxy settings from app config (same as Spotify imports)
        let proxy_url = self.get_proxy_from_config().await;
        
        let mut metadata_provider = if let Some(proxy) = proxy_url {
            crate::metadata::providers::MetadataProvider::new_with_proxy(Some(proxy))
        } else {
            crate::metadata::providers::MetadataProvider::new()
        };
        
        // Try to load API keys from config (if available)
        if let Ok(config) = crate::config::AppConfig::load() {
            if let (Some(client_id), Some(client_secret)) = (&config.api_keys.spotify_client_id, &config.api_keys.spotify_client_secret) {
                log::info!("Found Spotify credentials, getting access token...");
                match self.get_spotify_access_token(client_id, client_secret).await {
                    Ok(access_token) => {
                        log::info!("Successfully obtained Spotify access token");
                        metadata_provider.set_api_key("spotify", access_token);
                    }
                    Err(e) => {
                        log::warn!("Failed to get Spotify access token: {}", e);
                    }
                }
            } else {
                log::info!("No Spotify credentials found in config");
            }
        } else {
            log::warn!("Failed to load config for metadata search");
        }
        
        metadata_provider.search_metadata(&clean_artist, &clean_title).await
    }

    async fn get_proxy_from_config(&self) -> Option<String> {
        // Try to get proxy from environment variables first
        if let Ok(proxy) = std::env::var("HTTP_PROXY") {
            return Some(proxy);
        }
        if let Ok(proxy) = std::env::var("HTTPS_PROXY") {
            return Some(proxy);
        }
        if let Ok(proxy) = std::env::var("ALL_PROXY") {
            return Some(proxy);
        }
        
        // Use the same default proxy as Spotify imports (from config.rs)
        Some("http://127.0.0.1:1080".to_string())
    }

    async fn get_spotify_access_token(&self, client_id: &str, client_secret: &str) -> Result<String> {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()?;
        let params = [
            ("grant_type", "client_credentials"),
            ("client_id", client_id),
            ("client_secret", client_secret),
        ];

        let response = client
            .post("https://accounts.spotify.com/api/token")
            .form(&params)
            .send()
            .await?;

        if response.status().is_success() {
            let json: serde_json::Value = response.json().await?;
            if let Some(access_token) = json["access_token"].as_str() {
                Ok(access_token.to_string())
            } else {
                Err(crate::errors::AppError::ApiError("No access_token in Spotify response".to_string()))
            }
        } else {
            Err(crate::errors::AppError::ApiError(format!("Spotify token request failed: {}", response.status())))
        }
    }

    fn parse_track_info(&self, artist: &str, title: &str) -> (String, String) {
        // Common patterns for YouTube video titles that need parsing
        let patterns = vec![
            // Pattern: "Channel Name - Artist - Song (Lyrics)"
            (r"^[^-]+ - ([^-]+) - (.+?)(?:\s*\([^)]*\))?$", "artist_title"),
            // Pattern: "Artist - Song (Lyrics)" 
            (r"^([^-]+) - (.+?)(?:\s*\([^)]*\))?$", "artist_title"),
            // Pattern: "Song (Lyrics) - Artist"
            (r"^(.+?)(?:\s*\([^)]*\))? - ([^-]+)$", "title_artist"),
        ];

        // Check if the title contains artist information
        for (pattern, format) in &patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if let Some(captures) = regex.captures(title) {
                    match *format {
                        "artist_title" => {
                            if let (Some(artist_match), Some(title_match)) = (captures.get(1), captures.get(2)) {
                                let clean_artist = self.clean_track_name(artist_match.as_str());
                                let clean_title = self.clean_track_name(title_match.as_str());
                                log::info!("Parsed from title: {} - {}", clean_artist, clean_title);
                                return (clean_artist, clean_title);
                            }
                        }
                        "title_artist" => {
                            if let (Some(title_match), Some(artist_match)) = (captures.get(1), captures.get(2)) {
                                let clean_artist = self.clean_track_name(artist_match.as_str());
                                let clean_title = self.clean_track_name(title_match.as_str());
                                log::info!("Parsed from title: {} - {}", clean_artist, clean_title);
                                return (clean_artist, clean_title);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // If no pattern matches, clean the original values
        let clean_artist = self.clean_track_name(artist);
        let clean_title = self.clean_track_name(title);
        (clean_artist, clean_title)
    }

    fn clean_track_name(&self, name: &str) -> String {
        name
            .replace(" (Lyrics)", "")
            .replace(" (Official Video)", "")
            .replace(" (Official Audio)", "")
            .replace(" (Official)", "")
            .replace(" [Official Video]", "")
            .replace(" [Official Audio]", "")
            .replace(" [Official]", "")
            .replace(" - Lyrics", "")
            .replace(" - Official Video", "")
            .replace(" - Official Audio", "")
            .replace(" - Official", "")
            .replace(" (Music Video)", "")
            .replace(" [Music Video]", "")
            .replace(" (Audio)", "")
            .replace(" [Audio]", "")
            .trim()
            .to_string()
    }
}