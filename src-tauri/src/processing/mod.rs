// Processing modules removed - using Python for audio processing

use serde::{Deserialize, Serialize};
// Unused imports removed

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingOptions {
    pub input_format: String,
    pub output_format: String,
    pub quality: AudioQuality,
    pub bitrate: Option<u32>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AudioQuality {
    Low,    // 128 kbps
    Medium, // 192 kbps
    High,   // 256 kbps
    Best,   // 320 kbps
    Lossless, // FLAC/WAV
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingProgress {
    pub progress: f32,
    pub current_time: Option<f64>,
    pub total_time: Option<f64>,
    pub speed: Option<f64>,
    pub bitrate: Option<u32>,
}

// Transcoder trait removed - using Python for audio processing

impl AudioQuality {
    pub fn get_bitrate(&self) -> u32 {
        match self {
            AudioQuality::Low => 128,
            AudioQuality::Medium => 192,
            AudioQuality::High => 256,
            AudioQuality::Best => 320,
            AudioQuality::Lossless => 0, // Variable bitrate for lossless
        }
    }

    pub fn is_lossless(&self) -> bool {
        matches!(self, AudioQuality::Lossless)
    }
}
