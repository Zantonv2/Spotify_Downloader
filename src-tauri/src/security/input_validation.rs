use crate::errors::{AppError, Result};
use url::Url;
use std::path::Path;

pub struct InputValidator;

impl InputValidator {
    pub fn new() -> Self {
        Self
    }

    pub fn validate_url(&self, url: &str) -> Result<()> {
        if url.is_empty() {
            return Err(AppError::Validation("URL cannot be empty".to_string()));
        }

        let parsed_url = Url::parse(url)
            .map_err(|e| AppError::Validation(format!("Invalid URL: {}", e)))?;

        // Check if it's a supported platform
        let host = parsed_url.host_str()
            .ok_or_else(|| AppError::Validation("URL must have a host".to_string()))?;

        let supported_domains = [
            "youtube.com", "youtu.be", "soundcloud.com", "bandcamp.com", 
            "vimeo.com", "spotify.com", "open.spotify.com"
        ];

        let is_supported = supported_domains.iter().any(|domain| {
            host == *domain || host.ends_with(&format!(".{}", domain))
        });

        if !is_supported {
            return Err(AppError::Validation(format!(
                "Unsupported platform: {}. Supported platforms: YouTube, SoundCloud, Bandcamp, Vimeo, Spotify",
                host
            )));
        }

        Ok(())
    }

    pub fn validate_file_path(&self, path: &str) -> Result<()> {
        if path.is_empty() {
            return Err(AppError::Validation("File path cannot be empty".to_string()));
        }

        let path = Path::new(path);
        
        // Check for path traversal attempts
        if path.components().any(|component| {
            matches!(component, std::path::Component::ParentDir)
        }) {
            return Err(AppError::Validation("Path traversal detected".to_string()));
        }

        // Check for null bytes
        if path.to_string_lossy().contains('\0') {
            return Err(AppError::Validation("Null bytes not allowed in file path".to_string()));
        }

        Ok(())
    }

    pub fn validate_search_query(&self, query: &str) -> Result<()> {
        if query.is_empty() {
            return Err(AppError::Validation("Search query cannot be empty".to_string()));
        }

        if query.len() > 500 {
            return Err(AppError::Validation("Search query too long (max 500 characters)".to_string()));
        }

        // Check for potentially malicious content
        let dangerous_patterns = [
            "<script", "javascript:", "data:", "vbscript:",
            "onload=", "onerror=", "onclick=", "eval(",
            "document.cookie", "window.location"
        ];

        let query_lower = query.to_lowercase();
        for pattern in &dangerous_patterns {
            if query_lower.contains(pattern) {
                return Err(AppError::Validation(format!(
                    "Potentially malicious content detected: {}",
                    pattern
                )));
            }
        }

        Ok(())
    }

    pub fn validate_api_key(&self, service: &str, api_key: &str) -> Result<()> {
        if api_key.is_empty() {
            return Err(AppError::Validation("API key cannot be empty".to_string()));
        }

        if api_key.len() < 10 {
            return Err(AppError::Validation("API key too short (minimum 10 characters)".to_string()));
        }

        if api_key.len() > 1000 {
            return Err(AppError::Validation("API key too long (maximum 1000 characters)".to_string()));
        }

        // Validate service name
        let valid_services = ["spotify", "youtube", "soundcloud", "musicbrainz"];
        if !valid_services.contains(&service.to_lowercase().as_str()) {
            return Err(AppError::Validation(format!(
                "Invalid service: {}. Valid services: {}",
                service,
                valid_services.join(", ")
            )));
        }

        // Check for common patterns that might indicate invalid keys
        if api_key.chars().all(|c| c.is_whitespace()) {
            return Err(AppError::Validation("API key cannot be only whitespace".to_string()));
        }

        Ok(())
    }

    pub fn sanitize_filename(&self, filename: &str) -> Result<String> {
        if filename.is_empty() {
            return Err(AppError::Validation("Filename cannot be empty".to_string()));
        }

        // Remove or replace dangerous characters
        let sanitized = filename
            .chars()
            .map(|c| match c {
                '<' | '>' | ':' | '"' | '|' | '?' | '*' | '\\' | '/' => '_',
                '\0' | '\r' | '\n' | '\t' => '_',
                c if c.is_control() => '_',
                c => c,
            })
            .collect::<String>();

        // Remove leading/trailing dots and spaces
        let sanitized = sanitized.trim_matches(|c: char| c == '.' || c.is_whitespace());

        if sanitized.is_empty() {
            return Err(AppError::Validation("Filename becomes empty after sanitization".to_string()));
        }

        // Limit length
        if sanitized.len() > 255 {
            let truncated = &sanitized[..252];
            Ok(format!("{}...", truncated))
        } else {
            Ok(sanitized.to_string())
        }
    }

    pub fn validate_download_path(&self, path: &str) -> Result<()> {
        if path.is_empty() {
            return Err(AppError::Validation("Download path cannot be empty".to_string()));
        }

        let path = Path::new(path);
        
        // Check if path exists and is a directory
        if !path.exists() {
            return Err(AppError::Validation("Download path does not exist".to_string()));
        }

        if !path.is_dir() {
            return Err(AppError::Validation("Download path is not a directory".to_string()));
        }

        // Check for write permissions (simplified check)
        if !path.metadata()
            .map_err(|e| AppError::IoError(format!("Failed to read path metadata: {}", e)))?
            .permissions().readonly() {
            // This is a simplified check - in practice, you'd need to try creating a file
            Ok(())
        } else {
            Err(AppError::Validation("Download path is read-only".to_string()))
        }
    }
}
