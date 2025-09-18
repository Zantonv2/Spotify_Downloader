use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use crate::errors::Result;
use crate::downloader::{Downloader, DownloadTask, DownloadProgress};
use crate::downloader::http_pool::HttpPool;
use crate::downloader::cache::CacheManager;
use crate::metadata::MetadataInfo;
use crate::metadata::providers::MetadataProvider;
use crate::metadata::lyrics::LyricsProvider;
use crate::config::AppConfig;
use async_trait::async_trait;

/// yt-dlp information extractor
pub struct YtDlpExtractor {
    http_pool: Arc<HttpPool>,
    cache: Arc<CacheManager>,
    ytdlp_path: String,
}

impl YtDlpExtractor {
    pub fn new(http_pool: Arc<HttpPool>, cache: Arc<CacheManager>) -> Self {
        Self {
            http_pool,
            cache,
            ytdlp_path: "yt-dlp".to_string(),
        }
    }

    pub fn with_ytdlp_path(mut self, path: String) -> Self {
        self.ytdlp_path = path;
        self
    }

    /// Extract video information using yt-dlp
    pub async fn extract_info(&self, url: &str) -> Result<VideoInfo> {
        log::info!("🔍 [EXTRACT] Starting info extraction for URL: {}", url);
        
        // Check cache first
        let cache_key = format!("info:{}", url);
        log::info!("💾 [EXTRACT] Checking cache for key: {}", cache_key);
        if let Some(cached) = self.cache.metadata.get(&cache_key).await {
            log::info!("✅ [EXTRACT] Found cached info, using cached data");
            if let Ok(info) = serde_json::from_value::<VideoInfo>(cached) {
                return Ok(info);
            }
        }
        log::info!("❌ [EXTRACT] No cached info found, running yt-dlp");

        log::info!("🚀 [EXTRACT] Running yt-dlp command: {} --dump-json --no-warnings --ignore-errors --no-check-certificate {}", self.ytdlp_path, url);
        let cmd = Command::new(&self.ytdlp_path)
            .args(&[
                "--dump-json",
                "--no-warnings",
                "--ignore-errors",
                "--no-check-certificate",
                url,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        log::info!("⏳ [EXTRACT] Waiting for yt-dlp to complete...");
        let output = cmd.wait_with_output()?;
        log::info!("📊 [EXTRACT] yt-dlp exit status: {}", output.status);

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            log::error!("❌ [EXTRACT] yt-dlp extraction failed: {}", error);
            return Err(crate::errors::AppError::DownloadError(
                format!("yt-dlp extraction failed: {}", error)
            ));
        }

        log::info!("✅ [EXTRACT] yt-dlp completed successfully, parsing output...");
        let json_output = String::from_utf8(output.stdout)
            .map_err(|e| crate::errors::AppError::DownloadError(format!("Invalid UTF-8 in yt-dlp output: {}", e)))?;
        
        log::info!("📝 [EXTRACT] JSON output length: {} characters", json_output.len());
        let info: VideoInfo = serde_json::from_str(&json_output)?;
        log::info!("✅ [EXTRACT] Successfully parsed video info: {}", info.title);

        // Cache the result
        log::info!("💾 [EXTRACT] Caching extracted info...");
        self.cache.metadata.set(
            cache_key,
            serde_json::to_value(&info)?,
        ).await;

        log::info!("🎉 [EXTRACT] Info extraction completed successfully");
        Ok(info)
    }

    /// Search for videos using yt-dlp
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<VideoInfo>> {
        log::info!("🔍 [SEARCH] Starting search for: '{}' (limit: {})", query, limit);
        
        let cache_key = format!("search:{}:{}", query, limit);
        log::info!("💾 [SEARCH] Checking cache for key: {}", cache_key);
        if let Some(cached) = self.cache.metadata.get(&cache_key).await {
            log::info!("✅ [SEARCH] Found cached results, using cached data");
            if let Ok(results) = serde_json::from_value::<Vec<VideoInfo>>(cached) {
                return Ok(results);
            }
        }
        log::info!("❌ [SEARCH] No cached results found, running yt-dlp search");

        let search_query = format!("ytsearch{}:{}", limit, query);
        log::info!("🚀 [SEARCH] Running yt-dlp search command: {} --dump-json --flat-playlist --no-download --max-downloads {} {} --default-search ytsearch --ignore-errors --no-warnings --user-agent 'Mozilla/5.0...' --no-playlist --no-check-certificate --prefer-free-formats --extractor-retries 1 --fragment-retries 1", 
                   self.ytdlp_path, limit, search_query);
        
        let cmd = Command::new(&self.ytdlp_path)
            .args(&[
                "--dump-json",
                "--flat-playlist",
                "--no-download",
                "--max-downloads", &limit.to_string(),
                &search_query,
                "--default-search", "ytsearch",
                "--ignore-errors",
                "--no-warnings",
                "--user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
                "--no-playlist",
                "--no-check-certificate",
                "--extractor-retries", "1",
                "--fragment-retries", "1",
                "--socket-timeout", "10",
                "--retries", "1",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        log::info!("⏳ [SEARCH] Waiting for yt-dlp search to complete...");
        let output = cmd.wait_with_output()?;
        log::info!("📊 [SEARCH] yt-dlp search exit status: {}", output.status);

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            log::error!("❌ [SEARCH] yt-dlp search failed: {}", error);
            log::error!("📝 [SEARCH] stderr: {}", error);
            return Err(crate::errors::AppError::DownloadError(
                format!("yt-dlp search failed: {}", error)
            ));
        }

        log::info!("✅ [SEARCH] yt-dlp search completed successfully, parsing output...");
        let json_output = String::from_utf8(output.stdout)
            .map_err(|e| crate::errors::AppError::DownloadError(format!("Invalid UTF-8 in yt-dlp output: {}", e)))?;
        
        log::info!("📝 [SEARCH] JSON output length: {} characters", json_output.len());
        log::info!("📄 [SEARCH] Raw output: {}", json_output);
        
        let mut results = Vec::new();
        let lines: Vec<&str> = json_output.lines().collect();
        log::info!("📊 [SEARCH] Processing {} lines of output", lines.len());

        for (i, line) in lines.iter().enumerate() {
            log::info!("🔍 [SEARCH] Processing line {}: {}", i + 1, line);
            if let Ok(info) = serde_json::from_str::<VideoInfo>(line) {
                log::info!("✅ [SEARCH] Successfully parsed video: {}", info.title);
                results.push(info);
            } else {
                log::warn!("⚠️ [SEARCH] Failed to parse line {}: {}", i + 1, line);
            }
        }

        log::info!("📊 [SEARCH] Found {} valid results", results.len());

        // Cache the results
        log::info!("💾 [SEARCH] Caching search results...");
        self.cache.metadata.set(
            cache_key,
            serde_json::to_value(&results)?,
        ).await;

        log::info!("🎉 [SEARCH] Search completed successfully with {} results", results.len());
        Ok(results)
    }

    /// Get the best audio format URL
    pub async fn get_audio_url(&self, video_info: &VideoInfo, quality: &str) -> Result<String> {
        let format_id = self.select_audio_format(video_info, quality)?;
        
        // Use yt-dlp to get the direct URL
        let cmd = Command::new(&self.ytdlp_path)
            .args(&[
                "--get-url",
                "--format", &format_id,
                &video_info.webpage_url,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let output = cmd.wait_with_output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(crate::errors::AppError::DownloadError(
                format!("Failed to get audio URL: {}", error)
            ));
        }

        let url = String::from_utf8(output.stdout)
            .map_err(|e| crate::errors::AppError::DownloadError(format!("Invalid UTF-8 in yt-dlp URL output: {}", e)))?
            .trim()
            .to_string();
        Ok(url)
    }

    fn select_audio_format(&self, video_info: &VideoInfo, quality: &str) -> Result<String> {
        // For search results, formats might not be available
        // In that case, we'll use a default format selection
        if let Some(formats) = &video_info.formats {
            // Filter audio-only formats
            let audio_formats: Vec<_> = formats
                .iter()
                .filter(|f| f.acodec != "none" && f.vcodec == "none")
                .collect();

            if audio_formats.is_empty() {
                return Err(crate::errors::AppError::DownloadError(
                    "No audio-only formats available".to_string()
                ));
            }

            // Select format based on quality preference
            let selected = match quality {
                "best" => audio_formats.iter().max_by_key(|f| f.abr.unwrap_or(0)),
                "high" => audio_formats.iter()
                    .filter(|f| f.abr.unwrap_or(0) >= 192)
                    .max_by_key(|f| f.abr.unwrap_or(0)),
                "medium" => audio_formats.iter()
                    .filter(|f| f.abr.unwrap_or(0) >= 128 && f.abr.unwrap_or(0) < 192)
                    .max_by_key(|f| f.abr.unwrap_or(0)),
                "low" => audio_formats.iter()
                    .filter(|f| f.abr.unwrap_or(0) < 128)
                    .max_by_key(|f| f.abr.unwrap_or(0)),
                _ => audio_formats.iter().max_by_key(|f| f.abr.unwrap_or(0)),
            };

            match selected {
                Some(format) => Ok(format.format_id.clone()),
                None => Ok(audio_formats[0].format_id.clone()),
            }
        } else {
            // No formats available (search result), use default format selection
            // This will be handled by yt-dlp when we extract the actual video info
            Ok("bestaudio".to_string())
        }
    }
}

/// Video information from yt-dlp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoInfo {
    #[serde(rename = "_type")]
    pub _type: Option<String>,
    #[serde(rename = "ie_key")]
    pub ie_key: Option<String>,
    pub id: String,
    pub title: String,
    pub uploader: Option<String>,
    pub duration: Option<f64>,
    pub thumbnail: Option<String>,
    pub webpage_url: String,
    pub formats: Option<Vec<FormatInfo>>,
    pub upload_date: Option<String>,
    pub view_count: Option<u64>,
    pub like_count: Option<u64>,
    pub description: Option<String>,
    pub channel: Option<String>,
    pub channel_id: Option<String>,
    pub channel_url: Option<String>,
    pub uploader_id: Option<String>,
    pub uploader_url: Option<String>,
    pub thumbnails: Option<Vec<ThumbnailInfo>>,
    pub timestamp: Option<u64>,
    pub release_timestamp: Option<u64>,
    pub availability: Option<String>,
    pub live_status: Option<String>,
    pub channel_is_verified: Option<bool>,
    pub original_url: Option<String>,
    pub webpage_url_basename: Option<String>,
    pub webpage_url_domain: Option<String>,
    pub extractor: Option<String>,
    pub extractor_key: Option<String>,
    pub duration_string: Option<String>,
    pub release_year: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatInfo {
    pub format_id: String,
    pub ext: String,
    pub acodec: String,
    pub vcodec: String,
    pub abr: Option<u64>, // Audio bitrate
    pub vbr: Option<u64>, // Video bitrate
    pub filesize: Option<u64>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailInfo {
    pub url: String,
    pub height: Option<u64>,
    pub width: Option<u64>,
}

/// Rust-based yt-dlp downloader
pub struct RustYtDlpDownloader {
    name: String,
    extractor: YtDlpExtractor,
    http_pool: Arc<HttpPool>,
    cache: Arc<CacheManager>,
    active_downloads: Arc<Mutex<HashMap<String, DownloadProgress>>>,
    config: AppConfig,
}

impl RustYtDlpDownloader {
    pub fn new(http_pool: Arc<HttpPool>, cache: Arc<CacheManager>, config: AppConfig) -> Self {
        let extractor = YtDlpExtractor::new(http_pool.clone(), cache.clone());
        
        // Log GPU acceleration status
        if config.performance.ffmpeg_hardware_accel {
            let detected_gpu = crate::config::PerformanceConfig::detect_gpu_acceleration();
            match detected_gpu {
                crate::config::GpuAcceleration::Nvenc => log::info!("🚀 [GPU] NVIDIA GPU detected - CUDA acceleration enabled"),
                crate::config::GpuAcceleration::Qsv => log::info!("🚀 [GPU] Intel Quick Sync detected - QSV acceleration enabled"),
                crate::config::GpuAcceleration::Amf => log::info!("🚀 [GPU] AMD GPU detected - AMF acceleration enabled"),
                crate::config::GpuAcceleration::VideoToolbox => log::info!("🚀 [GPU] macOS VideoToolbox detected - hardware acceleration enabled"),
                crate::config::GpuAcceleration::None => log::info!("⚠️ [GPU] No GPU acceleration detected - using CPU only"),
                crate::config::GpuAcceleration::Auto => log::info!("🔍 [GPU] Auto-detecting GPU acceleration..."),
            }
        } else {
            log::info!("💻 [GPU] GPU acceleration disabled - using CPU only");
        }
        
        Self {
            name: "rust-ytdlp-downloader".to_string(),
            extractor,
            http_pool,
            cache,
            active_downloads: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    async fn download_audio_file(
        &self,
        url: &str,
        output_path: &Path,
        task_id: &str,
    ) -> Result<()> {
        // Add small random delay to prevent race conditions
        let delay_ms = (uuid::Uuid::new_v4().as_u128() % 100) as u64;
        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
        
        log::info!("⬇️ [RUST-YTDLP] Starting yt-dlp download...");
        log::info!("📁 [RUST-YTDLP] Output path: {:?}", output_path);
        log::info!("🔗 [RUST-YTDLP] URL: {}", url);
        
        // Create unique temp directory for this specific download to prevent race conditions
        let output_dir = output_path.parent()
            .ok_or_else(|| crate::errors::AppError::DownloadError("Invalid output path".to_string()))?;
        
        // Create a centralized temp directory
        let central_temp_dir = output_dir.join("temp");
        tokio::fs::create_dir_all(&central_temp_dir).await?;
        log::info!("📁 [TEMP] Central temp directory: {:?}", central_temp_dir);
        
        // Generate unique temp directory name with timestamp and UUIDs
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let unique_id = uuid::Uuid::new_v4();
        let temp_dir_name = format!("temp_{}_{}_{}", timestamp, task_id, unique_id);
        let temp_dir = central_temp_dir.join(temp_dir_name);
        tokio::fs::create_dir_all(&temp_dir).await?;
        log::info!("📁 [TEMP] Download temp directory: {:?}", temp_dir);
        
        // Generate temp filename with additional uniqueness
        let temp_filename = format!("audio_{}_{}.%(ext)s", task_id, uuid::Uuid::new_v4());
        let temp_file = temp_dir.join(temp_filename);
        
        // Find FFmpeg
        let ffmpeg_path = self.find_ffmpeg()?;
        let ffmpeg_dir = ffmpeg_path.parent()
            .ok_or_else(|| crate::errors::AppError::DownloadError("Invalid FFmpeg path".to_string()))?;
        
        // Build yt-dlp command using config settings
        let format = self.config.get_format_extension();
        let bitrate = self.config.get_quality_bitrate();
        
        // Build FFmpeg postprocessor args with GPU acceleration
        let mut ffmpeg_args = vec![
            format!("-c:a {}", if format == "m4a" { "aac" } else if format == "mp3" { "libmp3lame" } else { "copy" }),
            format!("-b:a {}k", bitrate),
            format!("-threads {}", self.config.performance.ffmpeg_threads),
        ];
        
        // Add GPU acceleration if enabled
        if self.config.performance.ffmpeg_hardware_accel {
            let gpu_args = self.config.performance.get_gpu_args();
            if !gpu_args.is_empty() {
                log::info!("🚀 [GPU] Using GPU acceleration: {:?}", gpu_args);
                ffmpeg_args.extend(gpu_args);
            }
        }
        
        let ffmpeg_args_str = ffmpeg_args.join(" ");
        
        let mut cmd = std::process::Command::new("yt-dlp");
        cmd.args(&[
            "--extract-audio",
            "--audio-format", format,
            "--audio-quality", "best",
            "--output", &temp_file.to_string_lossy(),
            "--no-playlist",
            "--no-warnings",
            "--ignore-errors",
            "--no-check-certificate",
            "--extractor-retries", "1",
            "--fragment-retries", "1",
            "--socket-timeout", &self.config.performance.socket_timeout.to_string(),
            "--retries", "1",
            "--concurrent-fragments", &self.config.performance.max_concurrent_fragments.to_string(),
            "--user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            "--postprocessor-args", &format!("ffmpeg:{}", ffmpeg_args_str),
            "--ffmpeg-location", &ffmpeg_dir.to_string_lossy(),
            url
        ]);
        
        log::info!("🎵 [RUST-YTDLP] Running yt-dlp command: {:?}", cmd);
        
        // Execute yt-dlp
        let output = cmd.output()?;
        
        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            log::error!("❌ [RUST-YTDLP] yt-dlp failed: {}", error_msg);
            return Err(crate::errors::AppError::DownloadError(format!("yt-dlp failed: {}", error_msg)));
        }
        
        // Find the downloaded file
        let downloaded_file = self.find_downloaded_file(&temp_dir).await?;
        
        // Move to final location with improved retry logic for file locking issues
        let mut retries = 10; // Increased retries
        let base_delay = 100; // Increased base delay
        let mut success = false;
        
        while retries > 0 && !success {
            match tokio::fs::rename(&downloaded_file, output_path).await {
                Ok(_) => {
                    success = true;
                    log::info!("✅ [RUST-YTDLP] File moved successfully to final location");
                },
                Err(e) if (e.kind() == std::io::ErrorKind::PermissionDenied || 
                          e.kind() == std::io::ErrorKind::AlreadyExists ||
                          e.kind() == std::io::ErrorKind::Other) && retries > 1 => {
                    let delay = base_delay + (10 - retries) * 100; // Exponential backoff with longer delays
                    log::warn!("⚠️ [RUST-YTDLP] File operation failed ({}), retrying in {}ms... ({} retries left)", e, delay, retries - 1);
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    retries -= 1;
                },
                Err(e) => {
                    log::error!("❌ [RUST-YTDLP] File operation failed permanently: {}", e);
                    return Err(e.into());
                }
            }
        }
        
        if !success {
            return Err(crate::errors::AppError::DownloadError("Failed to move file to final location after all retries".to_string()));
        }
        
        // Clean up temp directory (only the specific download's temp folder)
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
        
        log::info!("✅ [RUST-YTDLP] Audio file downloaded successfully: {:?}", output_path);
        Ok(())
    }
    
    fn find_ffmpeg(&self) -> Result<std::path::PathBuf> {
        // Try common FFmpeg locations
        let common_paths = [
            "C:\\Users\\temaz\\Downloads\\ffmpeg-master-latest-win64-gpl-shared\\bin\\ffmpeg.exe",
            "C:\\ffmpeg\\bin\\ffmpeg.exe",
            "C:\\Program Files\\ffmpeg\\bin\\ffmpeg.exe",
            "C:\\Program Files (x86)\\ffmpeg\\bin\\ffmpeg.exe",
        ];
        
        for path in &common_paths {
            if std::path::Path::new(path).exists() {
                return Ok(std::path::PathBuf::from(path));
            }
        }
        
        // Try PATH
        if let Ok(output) = std::process::Command::new("where").arg("ffmpeg").output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout);
                let path = path.lines().next().unwrap_or("").trim();
                if !path.is_empty() && std::path::Path::new(path).exists() {
                    return Ok(std::path::PathBuf::from(path));
                }
            }
        }
        
        Err(crate::errors::AppError::DownloadError("FFmpeg not found".to_string()))
    }
    
    async fn find_downloaded_file(&self, temp_dir: &std::path::Path) -> Result<std::path::PathBuf> {
        // Look for audio files in temp directory
        let expected_ext = self.config.get_format_extension();
        log::info!("🔍 [FIND] Looking for files with extension: {}", expected_ext);
        
        let mut entries = tokio::fs::read_dir(temp_dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_str().unwrap_or("");
                log::info!("🔍 [FIND] Found file: {:?} with extension: {}", path, ext_str);
                if ext_str == expected_ext {
                    log::info!("✅ [FIND] Found matching file: {:?}", path);
                    return Ok(path);
                }
            }
        }
        
        // Fallback: look for any audio file if the expected format wasn't found
        log::info!("⚠️ [FIND] No {} files found, looking for any audio file...", expected_ext);
        let mut entries = tokio::fs::read_dir(temp_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_str().unwrap_or("");
                if matches!(ext_str, "mp3" | "m4a" | "flac" | "wav" | "ogg" | "webm" | "opus") {
                    log::info!("✅ [FIND] Found fallback audio file: {:?} (extension: {})", path, ext_str);
                    return Ok(path);
                }
            }
        }
        
        Err(crate::errors::AppError::DownloadError("No audio file found after download".to_string()))
    }
    
    async fn search_metadata_parallel(&self, artist: &str, title: &str, album: Option<&str>) -> Result<Option<MetadataInfo>> {
        log::info!("🔍 [METADATA] Starting parallel metadata search for: {} - {}", artist, title);
        if let Some(album_name) = album {
            log::info!("📀 [METADATA] Album: {}", album_name);
        }
        
        let provider = MetadataProvider::new();
        
        // Use the enhanced search_metadata_with_album method which includes album information
        match provider.search_metadata_with_album(artist, title, album).await {
            Ok(Some(metadata)) => {
                log::info!("✅ [METADATA] Found enhanced metadata");
                Ok(Some(metadata))
            }
            Ok(None) => {
                log::warn!("⚠️ [METADATA] No enhanced metadata found from any source");
                Ok(None)
            }
            Err(e) => {
                log::warn!("⚠️ [METADATA] Metadata search failed: {}", e);
                Ok(None)
            }
        }
    }
    
    async fn search_lyrics_parallel(&self, artist: &str, title: &str) -> Result<Option<String>> {
        log::info!("🎵 [LYRICS] Starting parallel lyrics search for: {} - {}", artist, title);
        
        let provider = LyricsProvider::new();
        
        // Search for lyrics
        match provider.search_lyrics(artist, title).await {
            Ok(Some(lyrics)) => {
                log::info!("✅ [LYRICS] Found lyrics");
                Ok(Some(lyrics))
            }
            Ok(None) => {
                log::warn!("⚠️ [LYRICS] No lyrics found");
                Ok(None)
            }
            Err(e) => {
                log::warn!("⚠️ [LYRICS] Lyrics search failed: {}", e);
                Ok(None)
            }
        }
    }
    
    async fn embed_metadata(&self, file_path: &std::path::Path, metadata: &MetadataInfo, track_number: u32) -> Result<()> {
        log::info!("📝 [EMBED] Embedding metadata into: {:?}", file_path);
        log::info!("📝 [EMBED] Using track number: {}", track_number);
        
        // Convert path to proper UTF-8 string for Python processing
        let file_path_str = file_path.to_string_lossy().to_string();
        log::info!("📝 [EMBED] File path string: {}", file_path_str);
        
        // Verify file exists before trying to embed
        if !file_path.exists() {
            log::error!("❌ [EMBED] File does not exist: {:?}", file_path);
            return Err(crate::errors::AppError::DownloadError(format!("File does not exist: {:?}", file_path)));
        }
        
        // Call Python audio processor for metadata embedding
        let request = serde_json::json!({
            "action": "embed_metadata_only",
            "file_path": file_path_str,
            "metadata": {
                "title": metadata.title,
                "artist": metadata.artist,
                "album": metadata.album,
                "year": metadata.year,
                "genre": metadata.genre,
                "track_number": track_number, // Use the provided track number instead of metadata.track_number
                "disc_number": metadata.disc_number,
                "album_artist": metadata.album_artist,
                "composer": metadata.composer,
                "isrc": metadata.isrc,
                "cover_art_url": metadata.cover_art_url
            }
        });
        
        let result = self.call_python_processor(request).await?;
        
        if result.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
            log::info!("✅ [EMBED] Metadata embedded successfully");
        } else {
            let error_msg = result.get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            log::warn!("⚠️ [EMBED] Metadata embedding failed: {}", error_msg);
        }
        
        Ok(())
    }
    
    async fn embed_lyrics(&self, file_path: &std::path::Path, lyrics: &str) -> Result<()> {
        log::info!("🎵 [EMBED] Embedding lyrics into: {:?}", file_path);
        
        // Convert path to proper UTF-8 string for Python processing
        let file_path_str = file_path.to_string_lossy().to_string();
        log::info!("🎵 [EMBED] File path string: {}", file_path_str);
        
        // Verify file exists before trying to embed
        if !file_path.exists() {
            log::error!("❌ [EMBED] File does not exist: {:?}", file_path);
            return Err(crate::errors::AppError::DownloadError(format!("File does not exist: {:?}", file_path)));
        }
        
        // Call Python audio processor for lyrics embedding
        let request = serde_json::json!({
            "action": "embed_lyrics",
            "file_path": file_path_str,
            "lyrics": lyrics
        });
        
        let result = self.call_python_processor(request).await?;
        
        if result.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
            log::info!("✅ [EMBED] Lyrics embedded successfully");
        } else {
            let error_msg = result.get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            log::warn!("⚠️ [EMBED] Lyrics embedding failed: {}", error_msg);
        }
        
        Ok(())
    }
    
    async fn call_python_processor(&self, request: serde_json::Value) -> Result<serde_json::Value> {
        let mut cmd = std::process::Command::new("python");
        cmd.arg("../python_processor/audio_processor.py")
           .stdin(std::process::Stdio::piped())
           .stdout(std::process::Stdio::piped())
           .stderr(std::process::Stdio::piped())
           .env("PYTHONIOENCODING", "utf-8"); // Force UTF-8 encoding
        
        let mut child = cmd.spawn()?;
        
        // Send request to stdin
        if let Some(stdin) = child.stdin.take() {
            let request_str = serde_json::to_string(&request)?;
            log::info!("🐍 [PYTHON] Sending request to Python processor: {}", request_str);
            log::info!("🐍 [PYTHON] Request bytes (UTF-8): {:?}", request_str.as_bytes());
            tokio::task::spawn_blocking(move || {
                use std::io::Write;
                let mut stdin = stdin;
                stdin.write_all(request_str.as_bytes())?;
                stdin.flush()?;
                Ok::<(), std::io::Error>(())
            }).await.map_err(|e| crate::errors::AppError::DownloadError(format!("Failed to write to Python processor: {}", e)))??;
        }
        
        // Wait for completion
        let output = child.wait_with_output()?;
        
        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            log::error!("🐍 [PYTHON] Python processor failed: {}", error_msg);
            return Err(crate::errors::AppError::DownloadError(format!("Python processor failed: {}", error_msg)));
        }
        
        // Parse response
        let response_str = String::from_utf8_lossy(&output.stdout);
        log::info!("🐍 [PYTHON] Python processor response: {}", response_str);
        let response: serde_json::Value = serde_json::from_str(&response_str)?;
        
        Ok(response)
    }
    
    async fn extract_basic_metadata(&self, file_path: &std::path::Path, task: &DownloadTask) -> Result<MetadataInfo> {
        log::info!("📖 [BASIC-METADATA] Extracting basic metadata from: {:?}", file_path);
        
        // First, try to read existing metadata from the audio file
        let file_path_str = file_path.to_string_lossy().to_string();
        let request = serde_json::json!({
            "action": "read_metadata",
            "file_path": file_path_str
        });
        
        let result = self.call_python_processor(request).await?;
        
        if let Some(metadata) = result.get("metadata") {
            log::info!("📖 [BASIC-METADATA] Found existing metadata in audio file");
            let mut basic_metadata = MetadataInfo {
                title: metadata.get("title").and_then(|v| v.as_str()).unwrap_or(&task.track_info.title).to_string(),
                artist: metadata.get("artist").and_then(|v| v.as_str()).unwrap_or(&task.track_info.artist).to_string(),
                album: metadata.get("album").and_then(|v| v.as_str()).map(|s| s.to_string()).or(task.track_info.album.clone()),
                year: metadata.get("year").and_then(|v| v.as_u64()).map(|y| y as u32).or(task.track_info.year),
                genre: metadata.get("genre").and_then(|v| v.as_str()).map(|s| s.to_string()).or(task.track_info.genre.clone()),
                track_number: Some(task.order), // Always use queue position
                disc_number: metadata.get("disc_number").and_then(|v| v.as_u64()).map(|d| d as u32).or(task.track_info.disc_number),
                album_artist: metadata.get("album_artist").and_then(|v| v.as_str()).map(|s| s.to_string()).or(task.track_info.album_artist.clone()),
                composer: metadata.get("composer").and_then(|v| v.as_str()).map(|s| s.to_string()).or(task.track_info.composer.clone()),
                isrc: metadata.get("isrc").and_then(|v| v.as_str()).map(|s| s.to_string()).or(task.track_info.isrc.clone()),
                cover_art_url: task.track_info.thumbnail_url.clone(),
                lyrics: None,
            };
            
            // Ensure we have at least the basic info
            if basic_metadata.title.is_empty() {
                basic_metadata.title = task.track_info.title.clone();
            }
            if basic_metadata.artist.is_empty() {
                basic_metadata.artist = task.track_info.artist.clone();
            }
            
            return Ok(basic_metadata);
        }
        
        // Fallback: Create basic metadata from track info
        log::info!("📖 [BASIC-METADATA] No existing metadata found, creating from track info");
        Ok(MetadataInfo {
            title: task.track_info.title.clone(),
            artist: task.track_info.artist.clone(),
            album: task.track_info.album.clone(),
            year: task.track_info.year,
            genre: task.track_info.genre.clone(),
            track_number: Some(task.order),
            disc_number: task.track_info.disc_number,
            album_artist: task.track_info.album_artist.clone(),
            composer: task.track_info.composer.clone(),
            isrc: task.track_info.isrc.clone(),
            cover_art_url: task.track_info.thumbnail_url.clone(),
            lyrics: None,
        })
    }
}

#[async_trait]
impl Downloader for RustYtDlpDownloader {
    async fn download(&self, task: &DownloadTask) -> Result<()> {
        let start_time = std::time::Instant::now();
        log::info!("🎬 [RUST-YTDLP] Starting download: {} - {}", task.track_info.artist, task.track_info.title);
        log::info!("📁 [RUST-YTDLP] Output path: {:?}", task.output_path);
        log::info!("🆔 [RUST-YTDLP] Task ID: {}", task.id);
        
        // Start parallel metadata and lyrics searching IMMEDIATELY
        log::info!("🔍 [RUST-YTDLP] Starting parallel metadata and lyrics search...");
        let artist1 = task.track_info.artist.clone();
        let title1 = task.track_info.title.clone();
        let artist2 = task.track_info.artist.clone();
        let title2 = task.track_info.title.clone();
        let task_id1 = task.id.clone();
        let task_id2 = task.id.clone();
        let output_path = task.output_path.clone();
        
        let album1 = task.track_info.album.clone();
        let metadata_task = tokio::spawn(async move {
            log::info!("🔍 [METADATA-TASK-{}] Starting metadata search for: {} - {}", task_id1, artist1, title1);
            if let Some(ref album_name) = album1 {
                log::info!("📀 [METADATA-TASK-{}] Album: {}", task_id1, album_name);
            }
            // Create a new instance for the spawned task
            let provider = crate::metadata::providers::MetadataProvider::new();
            let result = provider.search_metadata_with_album(&artist1, &title1, album1.as_deref()).await;
            log::info!("🔍 [METADATA-TASK-{}] Metadata search completed", task_id1);
            result
        });
        let lyrics_task = tokio::spawn(async move {
            log::info!("🎵 [LYRICS-TASK-{}] Starting lyrics search for: {} - {}", task_id2, artist2, title2);
            // Create a new instance for the spawned task
            let provider = crate::metadata::lyrics::LyricsProvider::new();
            let result = provider.search_lyrics(&artist2, &title2).await;
            log::info!("🎵 [LYRICS-TASK-{}] Lyrics search completed", task_id2);
            result
        });
        
        let url = if !task.track_info.url.is_empty() {
            log::info!("🔗 [RUST-YTDLP] Using provided URL: {}", task.track_info.url);
            task.track_info.url.clone()
        } else {
            // Search for the track if no URL provided
            let query = format!("{} {}", task.track_info.artist, task.track_info.title);
            log::info!("🔍 [RUST-YTDLP] Searching for track: {}", query);
            let search_results = self.extractor.search(&query, 1).await?;
            
            if search_results.is_empty() {
                log::error!("❌ [RUST-YTDLP] No search results found for: {}", query);
                return Err(crate::errors::AppError::DownloadError(
                    "No search results found".to_string()
                ));
            }
            
            log::info!("✅ [RUST-YTDLP] Found {} search results", search_results.len());
            search_results[0].webpage_url.clone()
        };

        // Download directly with yt-dlp (like Python script)
        log::info!("⬇️ [RUST-YTDLP] Starting yt-dlp download...");
        self.download_audio_file(&url, &task.output_path, &task.id).await?;
        log::info!("✅ [RUST-YTDLP] Audio file download completed");

        // Wait for metadata and lyrics search to complete
        log::info!("⏳ [RUST-YTDLP] Waiting for metadata and lyrics search to complete...");
        let (metadata_result, lyrics_result) = tokio::join!(metadata_task, lyrics_task);
        
        // Unwrap the spawned task results
        let metadata_result = metadata_result.map_err(|e| crate::errors::AppError::DownloadError(format!("Metadata task failed: {}", e)))?;
        let lyrics_result = lyrics_result.map_err(|e| crate::errors::AppError::DownloadError(format!("Lyrics task failed: {}", e)))?;
        
        // Embed metadata and lyrics if found
        if let Some(metadata) = metadata_result? {
            log::info!("📝 [RUST-YTDLP] Embedding enhanced metadata...");
            self.embed_metadata(&output_path, &metadata, task.order).await?;
        } else {
            // Fallback: Try to extract basic metadata from the audio file itself
            log::info!("📝 [RUST-YTDLP] No enhanced metadata found, trying to extract basic metadata from audio file...");
            let basic_metadata = self.extract_basic_metadata(&output_path, &task).await?;
            self.embed_metadata(&output_path, &basic_metadata, task.order).await?;
        }
        
        if let Some(lyrics) = lyrics_result? {
            log::info!("🎵 [RUST-YTDLP] Embedding lyrics...");
            self.embed_lyrics(&output_path, &lyrics).await?;
        }

        let duration = start_time.elapsed();
        log::info!("🎉 [RUST-YTDLP] Successfully completed download: {} - {} (took {:.2} seconds)", 
                   task.track_info.artist, task.track_info.title, duration.as_secs_f64());
        Ok(())
    }

    async fn pause(&self, _task_id: &str) -> Result<()> {
        // Rust downloader doesn't support pausing mid-download
        Err(crate::errors::AppError::DownloadError("Pause not supported".to_string()))
    }

    async fn resume(&self, _task_id: &str) -> Result<()> {
        // Rust downloader doesn't support resuming
        Err(crate::errors::AppError::DownloadError("Resume not supported".to_string()))
    }

    async fn cancel(&self, task_id: &str) -> Result<()> {
        // Remove from active downloads
        let mut downloads = self.active_downloads.lock().await;
        downloads.remove(task_id);
        Ok(())
    }

    async fn get_progress(&self, task_id: &str) -> Result<DownloadProgress> {
        let downloads = self.active_downloads.lock().await;
        downloads.get(task_id)
            .cloned()
            .ok_or_else(|| crate::errors::AppError::DownloadError("Task not found".to_string()))
    }

    fn supports_format(&self, format: &str) -> bool {
        matches!(format, "mp3" | "m4a" | "flac" | "wav" | "ogg" | "webm" | "opus")
    }

    fn get_name(&self) -> &str {
        &self.name
    }
}
