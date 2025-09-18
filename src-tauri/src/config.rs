use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use dirs;
use crate::errors::{AppError, Result};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub download_path: PathBuf,
    pub max_concurrent_downloads: usize,
    pub preferred_quality: AudioQuality,
    pub preferred_format: AudioFormat,
    pub enable_metadata: bool,
    pub enable_lyrics: bool,
    pub enable_cover_art: bool,
    pub api_keys: ApiKeys,
    pub ui: UiConfig,
    pub proxy: Option<String>,
    pub performance: PerformanceConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiKeys {
    pub spotify_client_id: Option<String>,
    pub spotify_client_secret: Option<String>,
    pub musicbrainz_user_agent: Option<String>,
    pub youtube: Option<String>,
    pub soundcloud: Option<String>,
    pub musixmatch_client_id: Option<String>,
    pub musixmatch_client_secret: Option<String>,
    pub genius_client_id: Option<String>,
    pub genius_client_secret: Option<String>,
    pub deezer_api_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UiConfig {
    pub theme: String,
    pub show_notifications: bool,
    pub auto_start_downloads: bool,
    pub minimize_to_tray: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PerformanceConfig {
    pub enable_parallel_processing: bool,
    pub max_concurrent_fragments: u32,
    pub ffmpeg_threads: u32,
    pub socket_timeout: u32,
    pub enable_sponsorblock: bool,
    pub gpu_acceleration: GpuAcceleration,
    pub ffmpeg_hardware_accel: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum AudioQuality {
    Low,    // 128 kbps
    Medium, // 192 kbps
    High,   // 256 kbps
    Best,   // 320 kbps
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum AudioFormat {
    Mp3,
    M4a,
    Flac,
    Wav,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum GpuAcceleration {
    None,
    Nvenc,      // NVIDIA GPU
    Qsv,        // Intel Quick Sync Video
    Amf,        // AMD GPU
    VideoToolbox, // macOS
    Auto,       // Auto-detect best available
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            download_path: dirs::download_dir()
                .unwrap_or_else(|| PathBuf::from("./downloads")),
            max_concurrent_downloads: 3,
            preferred_quality: AudioQuality::High,
            preferred_format: AudioFormat::Mp3,
            enable_metadata: true,
            enable_lyrics: true,
            enable_cover_art: true,
            api_keys: ApiKeys::default(),
            ui: UiConfig::default(),
            proxy: Some("http://127.0.0.1:1080".to_string()),
            performance: PerformanceConfig::default(),
        }
    }
}

impl Default for ApiKeys {
    fn default() -> Self {
        Self {
            spotify_client_id: None,
            spotify_client_secret: None,
            musicbrainz_user_agent: Some("SpotifyDownloader/1.0".to_string()),
            youtube: None,
            soundcloud: None,
            musixmatch_client_id: None,
            musixmatch_client_secret: None,
            genius_client_id: None,
            genius_client_secret: None,
            deezer_api_key: None,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            show_notifications: true,
            auto_start_downloads: true,
            minimize_to_tray: false,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enable_parallel_processing: true,
            max_concurrent_fragments: 4,
            ffmpeg_threads: 4,
            socket_timeout: 15,
            enable_sponsorblock: false, // Disabled for speed
            gpu_acceleration: GpuAcceleration::Auto,
            ffmpeg_hardware_accel: true,
        }
    }
}

impl PerformanceConfig {
    /// Detect the best available GPU acceleration
    pub fn detect_gpu_acceleration() -> GpuAcceleration {
        log::info!("🔍 [GPU] Starting GPU detection...");
        
        // First, let's see what FFmpeg actually supports
        if let Ok(output) = std::process::Command::new("ffmpeg")
            .args(&["-hide_banner", "-encoders"])
            .output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            log::info!("🔍 [GPU] FFmpeg encoders available: {}", stdout);
        }
        // Check for NVIDIA GPU
        if std::process::Command::new("nvidia-smi")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false) {
            return GpuAcceleration::Nvenc;
        }
        
        // Check for Intel Quick Sync (Windows)
        if std::process::Command::new("ffmpeg")
            .args(&["-hide_banner", "-encoders"])
            .output()
            .map(|output| {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.contains("h264_qsv") || stdout.contains("hevc_qsv")
            })
            .unwrap_or(false) {
            return GpuAcceleration::Qsv;
        }
        
        // Check for AMD GPU - try multiple detection methods
        let amd_detected = std::process::Command::new("ffmpeg")
            .args(&["-hide_banner", "-encoders"])
            .output()
            .map(|output| {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Check for various AMD encoders
                stdout.contains("h264_amf") || 
                stdout.contains("hevc_amf") || 
                stdout.contains("av1_amf") ||
                stdout.contains("_amf")
            })
            .unwrap_or(false);
            
        if amd_detected {
            return GpuAcceleration::Amf;
        }
        
        // Alternative AMD detection - check for AMD hardware
        if std::process::Command::new("wmic")
            .args(&["path", "win32_VideoController", "get", "name"])
            .output()
            .map(|output| {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.to_lowercase().contains("amd") || 
                stdout.to_lowercase().contains("radeon") ||
                stdout.to_lowercase().contains("rx")
            })
            .unwrap_or(false) {
            log::info!("🔍 [GPU] AMD GPU detected via WMI, trying AMF anyway...");
            return GpuAcceleration::Amf;
        }
        
        // Final fallback - if we're on Windows and have a modern system, try AMD anyway
        #[cfg(target_os = "windows")]
        {
            log::info!("🔍 [GPU] No GPU detected via standard methods, trying AMD as fallback...");
            return GpuAcceleration::Amf;
        }
        
        #[cfg(not(target_os = "windows"))]
        GpuAcceleration::None
    }
    
    /// Get FFmpeg GPU acceleration arguments
    pub fn get_gpu_args(&self) -> Vec<String> {
        match self.gpu_acceleration {
            GpuAcceleration::Nvenc => vec![
                // NVIDIA hardware acceleration - minimal approach for audio
                "-hwaccel".to_string(),
                "cuda".to_string(),
            ],
            GpuAcceleration::Qsv => vec![
                // Intel QSV hardware acceleration - minimal approach for audio
                "-hwaccel".to_string(),
                "qsv".to_string(),
            ],
            GpuAcceleration::Amf => vec![
                // AMD hardware acceleration - disabled for audio-only conversion
                // "-hwaccel".to_string(),
                // "d3d11va".to_string(),
            ],
            GpuAcceleration::VideoToolbox => vec![
                // macOS VideoToolbox hardware acceleration - minimal approach for audio
                "-hwaccel".to_string(),
                "videotoolbox".to_string(),
            ],
            GpuAcceleration::Auto => {
                let detected = Self::detect_gpu_acceleration();
                match detected {
                    GpuAcceleration::Nvenc => vec![
                        "-hwaccel".to_string(),
                        "cuda".to_string(),
                    ],
                    GpuAcceleration::Qsv => vec![
                        "-hwaccel".to_string(),
                        "qsv".to_string(),
                    ],
                    GpuAcceleration::Amf => vec![
                        // AMD hardware acceleration - disabled for audio-only conversion
                        // "-hwaccel".to_string(),
                        // "d3d11va".to_string(),
                    ],
                    _ => vec![],
                }
            },
            GpuAcceleration::None => vec![],
        }
    }
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let config_path = Self::get_config_path()?;
        
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: AppConfig = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            let config = AppConfig::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path()?;
        let config_dir = config_path.parent().unwrap();
        
        if !config_dir.exists() {
            std::fs::create_dir_all(config_dir)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&config_path, content)?;
        Ok(())
    }

    fn get_config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| AppError::Config(config::ConfigError::Message("Could not find config directory".to_string())))?;
        
        Ok(config_dir.join("spotify-downloader").join("config.json"))
    }

    pub fn get_quality_bitrate(&self) -> u32 {
        match self.preferred_quality {
            AudioQuality::Low => 128,
            AudioQuality::Medium => 192,
            AudioQuality::High => 256,
            AudioQuality::Best => 320,
        }
    }

    pub fn get_format_extension(&self) -> &'static str {
        match self.preferred_format {
            AudioFormat::Mp3 => "mp3",
            AudioFormat::M4a => "m4a",
            AudioFormat::Flac => "flac",
            AudioFormat::Wav => "wav",
        }
    }
}
