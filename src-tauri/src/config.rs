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
