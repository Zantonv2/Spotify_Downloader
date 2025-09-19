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
use crate::metadata::{MetadataInfo, CoverArtData};
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
        
        let mut results = Vec::new();
        let lines: Vec<&str> = json_output.lines().collect();

        for line in lines.iter() {
            if let Ok(info) = serde_json::from_str::<VideoInfo>(line) {
                results.push(info);
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
                crate::config::GpuAcceleration::Nvenc => log::info!("🚀 NVIDIA GPU detected - CUDA acceleration enabled"),
                crate::config::GpuAcceleration::Qsv => log::info!("🚀 Intel Quick Sync detected - QSV acceleration enabled"),
                crate::config::GpuAcceleration::Amf => log::info!("🚀 AMD GPU detected - AMF acceleration enabled"),
                crate::config::GpuAcceleration::VideoToolbox => log::info!("🚀 macOS VideoToolbox detected - hardware acceleration enabled"),
                crate::config::GpuAcceleration::None => log::info!("⚠️ No GPU acceleration detected - using CPU only"),
                crate::config::GpuAcceleration::Auto => log::info!("🔍 Auto-detecting GPU acceleration..."),
            }
        } else {
            log::info!("💻 GPU acceleration disabled - using CPU only");
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
        
        log::info!("🎵 [CONFIG] Using audio format: {}", format);
        log::info!("🎵 [CONFIG] Using bitrate: {}k", bitrate);
        log::info!("🎵 [CONFIG] Using quality: {:?}", self.config.preferred_quality);
        
        // Build FFmpeg postprocessor args with GPU acceleration
        let mut ffmpeg_args = vec![
            format!("-threads {}", self.config.performance.ffmpeg_threads),
        ];
        
        // Add codec and quality settings based on format
        if format == "ogg" {
            match self.config.preferred_quality {
                crate::config::AudioQuality::Lossless => {
                    // Lossless OGG (FLAC in OGG container)
                    ffmpeg_args.push("-c:a".to_string());
                    ffmpeg_args.push("flac".to_string());
                    ffmpeg_args.push("-compression_level".to_string());
                    ffmpeg_args.push("5".to_string()); // Good balance of compression and speed
                    log::info!("🎵 [CONFIG] Using OGG FLAC (lossless)");
                },
                _ => {
                    // OGG Vorbis uses quality settings (0-10) instead of bitrate
                    ffmpeg_args.push("-c:a".to_string());
                    ffmpeg_args.push("libvorbis".to_string());
                    let vorbis_quality = match self.config.preferred_quality {
                        crate::config::AudioQuality::Low => "2",     // ~128kbps
                        crate::config::AudioQuality::Medium => "4",  // ~192kbps  
                        crate::config::AudioQuality::High => "6",    // ~256kbps
                        crate::config::AudioQuality::Best => "8",    // ~320kbps
                        _ => "6", // Default to high quality
                    };
                    ffmpeg_args.push(format!("-q:a {}", vorbis_quality));
                    log::info!("🎵 [CONFIG] Using OGG Vorbis quality: {}", vorbis_quality);
                }
            }
        } else if format == "opus" {
            // Opus uses bitrate settings
            ffmpeg_args.push("-c:a".to_string());
            ffmpeg_args.push("libopus".to_string());
            ffmpeg_args.push(format!("-b:a {}k", bitrate));
            log::info!("🎵 [CONFIG] Using Opus codec with {}k bitrate", bitrate);
        } else if format == "ape" {
            // APE is lossless only
            ffmpeg_args.push("-c:a".to_string());
            ffmpeg_args.push("ape".to_string());
            ffmpeg_args.push("-compression_level".to_string());
            ffmpeg_args.push("1000".to_string()); // High compression
            log::info!("🎵 [CONFIG] Using APE (lossless)");
        } else {
            // Other formats use standard codec and bitrate
            ffmpeg_args.push(format!("-c:a {}", if format == "m4a" { "aac" } else if format == "mp3" { "libmp3lame" } else if format == "flac" { "flac" } else if format == "wav" { "pcm_s16le" } else { "copy" }));
            ffmpeg_args.push(format!("-b:a {}k", bitrate));
        }
        
        // Add GPU acceleration if enabled
        if self.config.performance.ffmpeg_hardware_accel {
            let gpu_args = self.config.performance.get_gpu_args();
            if !gpu_args.is_empty() {
                log::info!("🚀 [GPU] Using GPU acceleration: {:?}", gpu_args);
                ffmpeg_args.extend(gpu_args);
            }
        }
        
        let ffmpeg_args_str = ffmpeg_args.join(" ");
        
        // Determine audio quality based on config
        let audio_quality = match self.config.preferred_quality {
            crate::config::AudioQuality::Best => "best",
            crate::config::AudioQuality::High => "high",
            crate::config::AudioQuality::Medium => "medium", 
            crate::config::AudioQuality::Low => "low",
            crate::config::AudioQuality::Lossless => "lossless",
        };
        
        let mut cmd = std::process::Command::new("yt-dlp");
        cmd.args(&[
            "--extract-audio",
            "--audio-format", format,
            "--audio-quality", audio_quality,
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
        
        // Validate the downloaded file (duration and size)
        self.validate_downloaded_file(&downloaded_file).await?;
        
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
        let expected_ext = self.config.get_format_extension();
        let mut entries = tokio::fs::read_dir(temp_dir).await?;
        
        // First try: look for expected format
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext.to_str().unwrap_or("") == expected_ext {
                    return Ok(path);
                }
            }
        }
        
        // Fallback: look for any audio file
        let mut entries = tokio::fs::read_dir(temp_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if matches!(ext.to_str().unwrap_or(""), "mp3" | "m4a" | "flac" | "wav" | "ogg" | "opus" | "ape" | "webm") {
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
        // Convert path to proper UTF-8 string for Python processing
        let file_path_str = file_path.to_string_lossy().to_string();
        
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

    async fn embed_cover_art(&self, file_path: &std::path::Path, cover_art: &crate::metadata::CoverArtInfo) -> Result<()> {
        log::info!("🖼️ [EMBED] Embedding cover art into: {:?}", file_path);
        log::info!("🖼️ [EMBED] Cover art URL: {}", cover_art.url);
        
        // Convert path to proper UTF-8 string for Python processing
        let file_path_str = file_path.to_string_lossy().to_string();
        log::info!("🖼️ [EMBED] File path string: {}", file_path_str);
        
        // Verify file exists before trying to embed
        if !file_path.exists() {
            log::error!("❌ [EMBED] File does not exist: {:?}", file_path);
            return Err(crate::errors::AppError::DownloadError(format!("File does not exist: {:?}", file_path)));
        }
        
        // Use pre-downloaded cover art data if available, otherwise download it
        let cover_art_data = if let Some(data) = &cover_art.data {
            log::info!("🖼️ [EMBED] Using pre-downloaded cover art data: {} bytes", data.len());
            crate::metadata::CoverArtData {
                data: data.clone(),
                mime_type: cover_art.mime_type.clone().unwrap_or_else(|| "image/jpeg".to_string()),
            }
        } else {
            log::info!("🖼️ [EMBED] No pre-downloaded data, downloading from URL: {}", cover_art.url);
            self.download_cover_art_data(&cover_art.url).await?
        };
        
        // Call Python audio processor for cover art embedding
        let request = serde_json::json!({
            "action": "embed_cover_art",
            "file_path": file_path_str,
            "cover_art": {
                "url": cover_art.url,
                "data": cover_art_data.data,
                "mime_type": cover_art_data.mime_type
            }
        });
        
        let result = self.call_python_processor(request).await?;
        
        if result.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
            log::info!("✅ [EMBED] Cover art embedded successfully");
        } else {
            let error_msg = result.get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            log::warn!("⚠️ [EMBED] Cover art embedding failed: {}", error_msg);
            
            // Don't fail the entire download if cover art fails
            // Just log the error and continue
        }
        
        Ok(())
    }

    async fn download_cover_art_data(&self, url: &str) -> Result<CoverArtData> {
        log::info!("🖼️ [DOWNLOAD] Downloading cover art from: {}", url);
        
        let response = self.http_pool.get_client()
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(crate::errors::AppError::DownloadError(format!("Failed to download cover art: HTTP {}", response.status())));
        }

        let mime_type = response.headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("image/jpeg")
            .to_string();
        let data = response.bytes().await?;

        log::info!("✅ [DOWNLOAD] Downloaded cover art: {} bytes, type: {}", data.len(), mime_type);
        
        Ok(CoverArtData {
            data: data.to_vec(),
            mime_type,
        })
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
        let response: serde_json::Value = serde_json::from_str(&response_str)?;
        
        Ok(response)
    }

    async fn validate_downloaded_file(&self, file_path: &std::path::Path) -> Result<()> {
        // Check file size (albums are usually >60MB)
        let metadata = tokio::fs::metadata(file_path).await?;
        let file_size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
        
        if file_size_mb > 60.0 {
            log::error!("❌ 5. File too large ({}MB) - likely an album, not a single track", file_size_mb);
            return Err(crate::errors::AppError::DownloadError(
                format!("File too large ({}MB) - likely an album, not a single track", file_size_mb)
            ));
        }
        
        // Check duration using FFmpeg
        let ffmpeg_path = self.find_ffmpeg()?;
        let mut cmd = std::process::Command::new(&ffmpeg_path);
        cmd.args(&[
            "-i", &file_path.to_string_lossy(),
            "-f", "null",
            "-"
        ]);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        
        let output = cmd.output()?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        // Parse duration from FFmpeg output
        if let Some(duration_line) = stderr.lines().find(|line| line.contains("Duration:")) {
            if let Some(duration_str) = duration_line.split("Duration: ").nth(1) {
                if let Some(duration_part) = duration_str.split(',').next() {
                    // Parse duration in format HH:MM:SS.mmm
                    let parts: Vec<&str> = duration_part.split(':').collect();
                    if parts.len() == 3 {
                        let hours: u32 = parts[0].parse().unwrap_or(0);
                        let minutes: u32 = parts[1].parse().unwrap_or(0);
                        let seconds: f64 = parts[2].parse().unwrap_or(0.0);
                        
                        let total_seconds = hours as f64 * 3600.0 + minutes as f64 * 60.0 + seconds;
                        let total_minutes = total_seconds / 60.0;
                        
                        if total_minutes > 20.0 {
                            log::error!("❌ 5. Track too long ({:.1} minutes) - likely an album or compilation", total_minutes);
                            return Err(crate::errors::AppError::DownloadError(
                                format!("Track too long ({:.1} minutes) - likely an album or compilation", total_minutes)
                            ));
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    async fn verify_metadata_embedding(&self, file_path: &std::path::Path, task: &DownloadTask) -> Result<()> {
        // Read the embedded metadata from the file
        let file_path_str = file_path.to_string_lossy().to_string();
        let request = serde_json::json!({
            "action": "read_metadata",
            "file_path": file_path_str
        });
        
        let result = self.call_python_processor(request).await?;
        
        if let Some(metadata) = result.get("metadata") {
            let embedded_title = metadata.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let embedded_artist = metadata.get("artist").and_then(|v| v.as_str()).unwrap_or("");
            let embedded_track_number = metadata.get("track_number").and_then(|v| v.as_u64()).map(|n| n as u32);
            
            // Check if critical metadata is present
            let mut issues = Vec::new();
            
            if embedded_title.is_empty() || embedded_title == "Unknown Title" {
                issues.push("Missing or invalid title");
            }
            
            if embedded_artist.is_empty() || embedded_artist == "Unknown Artist" {
                issues.push("Missing or invalid artist");
            }
            
            if embedded_track_number.is_none() {
                issues.push("Missing track number");
            }
            
            if !issues.is_empty() {
                log::error!("❌ 6. Metadata verification failed: {}", issues.join(", "));
                
                // Try to fix missing track number
                if embedded_track_number.is_none() {
                    let embedded_album = metadata.get("album").and_then(|v| v.as_str()).unwrap_or("");
                    let embedded_year = metadata.get("year").and_then(|v| v.as_u64()).map(|y| y as u32);
                    let embedded_genre = metadata.get("genre").and_then(|v| v.as_str()).unwrap_or("");
                    
                    let fix_request = serde_json::json!({
                        "action": "embed_metadata",
                        "file_path": file_path_str,
                        "metadata": {
                            "title": embedded_title,
                            "artist": embedded_artist,
                            "album": embedded_album,
                            "year": embedded_year,
                            "genre": embedded_genre,
                            "track_number": task.order,
                            "disc_number": metadata.get("disc_number").and_then(|v| v.as_u64()).map(|n| n as u32),
                            "album_artist": metadata.get("album_artist").and_then(|v| v.as_str()),
                            "composer": metadata.get("composer").and_then(|v| v.as_str()),
                            "isrc": metadata.get("isrc").and_then(|v| v.as_str()),
                            "cover_art_url": metadata.get("cover_art_url").and_then(|v| v.as_str()),
                            "lyrics": metadata.get("lyrics").and_then(|v| v.as_str())
                        }
                    });
                    
                    let _ = self.call_python_processor(fix_request).await;
                }
            }
        } else {
            log::error!("❌ 6. No metadata found in file - embedding may have failed");
        }
        
        Ok(())
    }

    async fn validate_flac_metadata(&self, file_path: &std::path::Path) -> Result<()> {
        log::info!("🔍 [FLAC-VALIDATE] Validating FLAC metadata for: {:?}", file_path);
        
        let file_path_str = file_path.to_string_lossy().to_string();
        let request = serde_json::json!({
            "action": "validate_flac_metadata",
            "file_path": file_path_str
        });
        
        let result = self.call_python_processor(request).await?;
        
        if let Some(success) = result.get("success").and_then(|v| v.as_bool()) {
            if success {
                log::info!("✅ [FLAC-VALIDATE] FLAC metadata validation passed");
                
                // Log additional validation details
                if let Some(cover_art_present) = result.get("cover_art_present").and_then(|v| v.as_bool()) {
                    if cover_art_present {
                        log::info!("🖼️ [FLAC-VALIDATE] Cover art present");
                    } else {
                        log::warn!("⚠️ [FLAC-VALIDATE] No cover art found");
                    }
                }
                
                if let Some(lyrics_present) = result.get("lyrics_present").and_then(|v| v.as_bool()) {
                    if lyrics_present {
                        log::info!("🎵 [FLAC-VALIDATE] Lyrics present");
                    } else {
                        log::info!("ℹ️ [FLAC-VALIDATE] No lyrics found");
                    }
                }
                
                if let Some(enhanced_present) = result.get("enhanced_metadata_present").and_then(|v| v.as_bool()) {
                    if enhanced_present {
                        log::info!("✨ [FLAC-VALIDATE] Enhanced metadata present");
                    } else {
                        log::info!("ℹ️ [FLAC-VALIDATE] Basic metadata only");
                    }
                }
                
                if let Some(vorbis_count) = result.get("vorbis_comments_count").and_then(|v| v.as_u64()) {
                    log::info!("📊 [FLAC-VALIDATE] {} Vorbis comments embedded", vorbis_count);
                }
                
                if let Some(file_size) = result.get("file_size_mb").and_then(|v| v.as_f64()) {
                    log::info!("📁 [FLAC-VALIDATE] File size: {:.1} MB", file_size);
                }
            } else {
                if let Some(missing_fields) = result.get("missing_required_fields").and_then(|v| v.as_array()) {
                    let fields: Vec<String> = missing_fields.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    log::error!("❌ [FLAC-VALIDATE] Missing required fields: {}", fields.join(", "));
                }
                log::error!("❌ [FLAC-VALIDATE] FLAC metadata validation failed");
            }
        } else {
            log::warn!("⚠️ [FLAC-VALIDATE] Could not determine validation result");
        }
        
        Ok(())
    }

    async fn validate_wav_metadata(&self, file_path: &std::path::Path) -> Result<()> {
        log::info!("🔍 [WAV-VALIDATE] Validating WAV metadata for: {:?}", file_path);
        
        let file_path_str = file_path.to_string_lossy().to_string();
        let request = serde_json::json!({
            "action": "validate_wav_metadata",
            "file_path": file_path_str
        });
        
        let result = self.call_python_processor(request).await?;
        
        if let Some(success) = result.get("success").and_then(|v| v.as_bool()) {
            if success {
                log::info!("✅ [WAV-VALIDATE] WAV metadata validation passed");
                
                // Log additional validation details
                if let Some(cover_art_present) = result.get("cover_art_present").and_then(|v| v.as_bool()) {
                    if cover_art_present {
                        log::info!("🖼️ [WAV-VALIDATE] Cover art present (APIC frame)");
                    } else {
                        log::info!("ℹ️ [WAV-VALIDATE] No embedded cover art (external cover art may be available)");
                    }
                }
                
                if let Some(lyrics_present) = result.get("lyrics_present").and_then(|v| v.as_bool()) {
                    if lyrics_present {
                        log::info!("🎵 [WAV-VALIDATE] Lyrics present (USLT frame)");
                    } else {
                        log::info!("ℹ️ [WAV-VALIDATE] No lyrics found");
                    }
                }
                
                if let Some(enhanced_present) = result.get("enhanced_metadata_present").and_then(|v| v.as_bool()) {
                    if enhanced_present {
                        log::info!("✨ [WAV-VALIDATE] Enhanced metadata present");
                    } else {
                        log::info!("ℹ️ [WAV-VALIDATE] Basic metadata only");
                    }
                }
                
                if let Some(id3_count) = result.get("total_id3_tags").and_then(|v| v.as_u64()) {
                    log::info!("📊 [WAV-VALIDATE] {} ID3v2 tags embedded", id3_count);
                }
                
                if let Some(file_size) = result.get("file_size_mb").and_then(|v| v.as_f64()) {
                    log::info!("📁 [WAV-VALIDATE] File size: {:.1} MB", file_size);
                }
            } else {
                if let Some(missing_fields) = result.get("missing_required_fields").and_then(|v| v.as_array()) {
                    let fields: Vec<String> = missing_fields.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    log::error!("❌ [WAV-VALIDATE] Missing required fields: {}", fields.join(", "));
                }
                log::error!("❌ [WAV-VALIDATE] WAV metadata validation failed");
            }
        } else {
            log::warn!("⚠️ [WAV-VALIDATE] Could not determine validation result");
        }
        
        Ok(())
    }

    async fn validate_ogg_metadata(&self, file_path: &std::path::Path) -> Result<()> {
        log::info!("🔍 [OGG-VALIDATE] Validating OGG metadata for: {:?}", file_path);
        
        let file_path_str = file_path.to_string_lossy().to_string();
        let request = serde_json::json!({
            "action": "validate_ogg_metadata",
            "file_path": file_path_str
        });
        
        let result = self.call_python_processor(request).await?;
        
        if let Some(success) = result.get("success").and_then(|v| v.as_bool()) {
            if success {
                log::info!("✅ [OGG-VALIDATE] OGG metadata validation passed");
                
                // Log additional validation details
                if let Some(cover_art_present) = result.get("cover_art_present").and_then(|v| v.as_bool()) {
                    if cover_art_present {
                        log::info!("🖼️ [OGG-VALIDATE] Cover art present (Vorbis comment)");
                    } else {
                        log::info!("ℹ️ [OGG-VALIDATE] No cover art found");
                    }
                }
                
                if let Some(lyrics_present) = result.get("lyrics_present").and_then(|v| v.as_bool()) {
                    if lyrics_present {
                        log::info!("🎵 [OGG-VALIDATE] Lyrics present (Vorbis comment)");
                    } else {
                        log::info!("ℹ️ [OGG-VALIDATE] No lyrics found");
                    }
                }
                
                if let Some(enhanced_present) = result.get("enhanced_metadata_present").and_then(|v| v.as_bool()) {
                    if enhanced_present {
                        log::info!("✨ [OGG-VALIDATE] Enhanced metadata present");
                    } else {
                        log::info!("ℹ️ [OGG-VALIDATE] Basic metadata only");
                    }
                }
                
                if let Some(comment_count) = result.get("total_vorbis_comments").and_then(|v| v.as_u64()) {
                    log::info!("📊 [OGG-VALIDATE] {} Vorbis comments embedded", comment_count);
                }
                
                if let Some(file_size) = result.get("file_size_mb").and_then(|v| v.as_f64()) {
                    log::info!("📁 [OGG-VALIDATE] File size: {:.1} MB", file_size);
                }
            } else {
                if let Some(missing_fields) = result.get("missing_required_fields").and_then(|v| v.as_array()) {
                    let fields: Vec<String> = missing_fields.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    log::error!("❌ [OGG-VALIDATE] Missing required fields: {}", fields.join(", "));
                }
                log::error!("❌ [OGG-VALIDATE] OGG metadata validation failed");
            }
        } else {
            log::warn!("⚠️ [OGG-VALIDATE] Could not determine validation result");
        }
        
        Ok(())
    }

    async fn validate_opus_metadata(&self, file_path: &std::path::Path) -> Result<()> {
        log::info!("🔍 [OPUS-VALIDATE] Validating Opus metadata for: {:?}", file_path);
        
        let file_path_str = file_path.to_string_lossy().to_string();
        let request = serde_json::json!({
            "action": "validate_opus_metadata",
            "file_path": file_path_str
        });
        
        let result = self.call_python_processor(request).await?;
        
        if let Some(success) = result.get("success").and_then(|v| v.as_bool()) {
            if success {
                log::info!("✅ [OPUS-VALIDATE] Opus metadata validation passed");
                
                // Log additional validation details
                if let Some(cover_art_present) = result.get("cover_art_present").and_then(|v| v.as_bool()) {
                    if cover_art_present {
                        log::info!("🖼️ [OPUS-VALIDATE] Cover art present (Vorbis comment)");
                    } else {
                        log::info!("ℹ️ [OPUS-VALIDATE] No cover art found");
                    }
                }
                
                if let Some(lyrics_present) = result.get("lyrics_present").and_then(|v| v.as_bool()) {
                    if lyrics_present {
                        log::info!("🎵 [OPUS-VALIDATE] Lyrics present (Vorbis comment)");
                    } else {
                        log::info!("ℹ️ [OPUS-VALIDATE] No lyrics found");
                    }
                }
                
                if let Some(comment_count) = result.get("total_vorbis_comments").and_then(|v| v.as_u64()) {
                    log::info!("📊 [OPUS-VALIDATE] {} Vorbis comments embedded", comment_count);
                }
                
                if let Some(file_size) = result.get("file_size_mb").and_then(|v| v.as_f64()) {
                    log::info!("📁 [OPUS-VALIDATE] File size: {:.1} MB", file_size);
                }
            } else {
                if let Some(missing_fields) = result.get("missing_required_fields").and_then(|v| v.as_array()) {
                    let fields: Vec<String> = missing_fields.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    log::error!("❌ [OPUS-VALIDATE] Missing required fields: {}", fields.join(", "));
                }
                log::error!("❌ [OPUS-VALIDATE] Opus metadata validation failed");
            }
        } else {
            log::warn!("⚠️ [OPUS-VALIDATE] Could not determine validation result");
        }
        
        Ok(())
    }

    async fn validate_ape_metadata(&self, file_path: &std::path::Path) -> Result<()> {
        log::info!("🔍 [APE-VALIDATE] Validating APE metadata for: {:?}", file_path);
        
        let file_path_str = file_path.to_string_lossy().to_string();
        let request = serde_json::json!({
            "action": "validate_ape_metadata",
            "file_path": file_path_str
        });
        
        let result = self.call_python_processor(request).await?;
        
        if let Some(success) = result.get("success").and_then(|v| v.as_bool()) {
            if success {
                log::info!("✅ [APE-VALIDATE] APE metadata validation passed");
                
                // Log additional validation details
                if let Some(cover_art_present) = result.get("cover_art_present").and_then(|v| v.as_bool()) {
                    if cover_art_present {
                        log::info!("🖼️ [APE-VALIDATE] Cover art present (APEv2)");
                    } else {
                        log::info!("ℹ️ [APE-VALIDATE] No cover art found");
                    }
                }
                
                if let Some(lyrics_present) = result.get("lyrics_present").and_then(|v| v.as_bool()) {
                    if lyrics_present {
                        log::info!("🎵 [APE-VALIDATE] Lyrics present (APEv2)");
                    } else {
                        log::info!("ℹ️ [APE-VALIDATE] No lyrics found");
                    }
                }
                
                if let Some(tag_count) = result.get("total_ape_tags").and_then(|v| v.as_u64()) {
                    log::info!("📊 [APE-VALIDATE] {} APEv2 tags embedded", tag_count);
                }
                
                if let Some(file_size) = result.get("file_size_mb").and_then(|v| v.as_f64()) {
                    log::info!("📁 [APE-VALIDATE] File size: {:.1} MB", file_size);
                }
            } else {
                if let Some(missing_fields) = result.get("missing_required_fields").and_then(|v| v.as_array()) {
                    let fields: Vec<String> = missing_fields.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    log::error!("❌ [APE-VALIDATE] Missing required fields: {}", fields.join(", "));
                }
                log::error!("❌ [APE-VALIDATE] APE metadata validation failed");
            }
        } else {
            log::warn!("⚠️ [APE-VALIDATE] Could not determine validation result");
        }
        
        Ok(())
    }
    
    async fn extract_basic_metadata(&self, file_path: &std::path::Path, task: &DownloadTask) -> Result<MetadataInfo> {
        
        // First, try to read existing metadata from the audio file
        let file_path_str = file_path.to_string_lossy().to_string();
        let request = serde_json::json!({
            "action": "read_metadata",
            "file_path": file_path_str
        });
        
        let result = self.call_python_processor(request).await?;
        
        if let Some(metadata) = result.get("metadata") {
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
        // Line 1: Searching for track
        log::info!("🔍 1. Searching for track \"{}\" - \"{}\"", task.track_info.artist, task.track_info.title);
        
        // Line 2: Searching for metadata and lyrics
        log::info!("📊 2. Searching for metadata and lyrics");
        
        // Start parallel metadata, lyrics, and cover art searching
        let artist1 = task.track_info.artist.clone();
        let title1 = task.track_info.title.clone();
        let artist2 = task.track_info.artist.clone();
        let title2 = task.track_info.title.clone();
        let artist3 = task.track_info.artist.clone();
        let title3 = task.track_info.title.clone();
        let album1 = task.track_info.album.clone();
        let album3 = task.track_info.album.clone();
        
        let metadata_task = tokio::spawn(async move {
            let provider = crate::metadata::providers::MetadataProvider::new();
            provider.search_metadata_with_album(&artist1, &title1, album1.as_deref()).await
        });
        let lyrics_task = tokio::spawn(async move {
            let provider = crate::metadata::lyrics::LyricsProvider::new();
            provider.search_lyrics(&artist2, &title2).await
        });
        let cover_art_task = tokio::spawn(async move {
            let provider = crate::metadata::providers::MetadataProvider::new();
            provider.search_cover_art(&artist3, &title3, album3.as_deref()).await
        });
        
        let url = if !task.track_info.url.is_empty() {
            task.track_info.url.clone()
        } else {
            // Search for the track if no URL provided
            let query = format!("{} {}", task.track_info.artist, task.track_info.title);
            let search_results = self.extractor.search(&query, 1).await?;
            
            if search_results.is_empty() {
                log::error!("❌ 1. No search results found for: {}", query);
                return Err(crate::errors::AppError::DownloadError(
                    "No search results found".to_string()
                ));
            }
            
            search_results[0].webpage_url.clone()
        };

        // Line 3: Downloading track
        log::info!("⬇️ 3. Downloading track");
        self.download_audio_file(&url, &task.output_path, &task.id).await?;

        // Wait for metadata, lyrics, and cover art search to complete
        let (metadata_result, lyrics_result, cover_art_result) = tokio::join!(metadata_task, lyrics_task, cover_art_task);
        
        // Unwrap the spawned task results
        let metadata_result = metadata_result.map_err(|e| crate::errors::AppError::DownloadError(format!("Metadata task failed: {}", e)))?;
        let lyrics_result = lyrics_result.map_err(|e| crate::errors::AppError::DownloadError(format!("Lyrics task failed: {}", e)))?;
        let cover_art_result = cover_art_result.map_err(|e| crate::errors::AppError::DownloadError(format!("Cover art task failed: {}", e)))?;
        
        // Line 4: Found metadata
        let metadata_found = metadata_result.is_ok() && metadata_result.as_ref().unwrap().is_some();
        let lyrics_found = lyrics_result.is_ok() && lyrics_result.as_ref().unwrap().is_some();
        let cover_art_found = cover_art_result.is_ok() && cover_art_result.as_ref().unwrap().is_some();
        let mut metadata_info = String::new();
        if metadata_found {
            metadata_info.push_str("metadata");
        }
        if lyrics_found {
            if !metadata_info.is_empty() {
                metadata_info.push_str(" + ");
            }
            metadata_info.push_str("lyrics");
        }
        if cover_art_found {
            if !metadata_info.is_empty() {
                metadata_info.push_str(" + ");
            }
            metadata_info.push_str("cover art");
        }
        if metadata_info.is_empty() {
            metadata_info.push_str("none");
        }
        log::info!("✅ 4. Found metadata: {}", metadata_info);
        
        // Line 5: Downloaded track
        log::info!("📁 5. Downloaded track");
        
        // Line 6: Embedding metadata
        log::info!("📝 6. Embedding metadata");
        
        // Embed metadata and lyrics if found
        if let Ok(Some(mut metadata)) = metadata_result {
            // Use cover art from separate search if available
            if let Ok(Some(ref cover_art)) = cover_art_result {
                metadata.cover_art_url = Some(cover_art.url.clone());
            }
            self.embed_metadata(&task.output_path, &metadata, task.order).await?;
        } else {
            // Fallback: Try to extract basic metadata from the audio file itself
            let mut basic_metadata = self.extract_basic_metadata(&task.output_path, &task).await?;
            // Use cover art from separate search if available
            if let Ok(Some(ref cover_art)) = cover_art_result {
                basic_metadata.cover_art_url = Some(cover_art.url.clone());
            }
            self.embed_metadata(&task.output_path, &basic_metadata, task.order).await?;
        }

        // Use cover art from the parallel search
        let cover_art_to_embed = if let Ok(Some(cover_art)) = cover_art_result {
            Some(cover_art)
        } else {
            None
        };
        
        if let Ok(Some(lyrics)) = lyrics_result {
            self.embed_lyrics(&task.output_path, &lyrics).await?;
        }

        // Embed cover art if found
        if let Some(cover_art) = cover_art_to_embed {
            self.embed_cover_art(&task.output_path, &cover_art).await?;
        }

        // Verify metadata was embedded correctly
        self.verify_metadata_embedding(&task.output_path, &task).await?;
        
        // Additional format-specific validation
        if let Some(ext) = task.output_path.extension().and_then(|s| s.to_str()) {
            match ext {
                "flac" => self.validate_flac_metadata(&task.output_path).await?,
                "wav" => self.validate_wav_metadata(&task.output_path).await?,
                "ogg" => self.validate_ogg_metadata(&task.output_path).await?,
                "opus" => self.validate_opus_metadata(&task.output_path).await?,
                "ape" => self.validate_ape_metadata(&task.output_path).await?,
                _ => {} // No specific validation for other formats
            }
        }

        // Line 7: Download completed
        log::info!("🎉 7. Download completed for \"{}\" - \"{}\"", task.track_info.artist, task.track_info.title);
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
