use std::process::{Command, Stdio};
use std::io::Write;
use serde_json::Value;
use crate::errors::{AppError, Result};
use log::{info, error};

// execute_python_script removed - using execute_python_script_with_ffmpeg instead

/// Executes a Python subprocess with FFmpeg environment variable set
pub async fn execute_python_script_with_ffmpeg(script_path: &str, input_data: Value, ffmpeg_path: Option<String>) -> Result<Value> {
    let mut cmd = Command::new("python");
    cmd.arg(script_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    
    // Set FFMPEG_BINARY environment variable if FFmpeg path is provided
    if let Some(path) = ffmpeg_path {
        cmd.env("FFMPEG_BINARY", &path);
    }
    
    let mut child = cmd.spawn()
        .map_err(|e| AppError::PythonSubprocess(format!("Failed to start Python process: {}", e)))?;

    // Send input data to Python script
    if let Some(stdin) = child.stdin.as_mut() {
        let input_json = serde_json::to_string(&input_data)?;
        stdin.write_all(input_json.as_bytes())?;
        stdin.write_all(b"\n")?; // Ensure the input is terminated
    }

    // Wait for the process to complete
    let output = child.wait_with_output()
        .map_err(|e| AppError::PythonSubprocess(format!("Failed to wait for Python process: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Python script failed: {}", stderr);
        return Err(AppError::PythonSubprocess(format!("Script execution failed: {}", stderr)));
    }

    // Parse the JSON response
    let stdout = String::from_utf8_lossy(&output.stdout);
    let response: Value = serde_json::from_str(&stdout)
        .map_err(|e| AppError::Json(e))?;

    Ok(response)
}

// validate_url removed - not used

/// Sanitizes a filename by removing invalid characters
pub fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect()
}

/// Sanitizes track filename in "Artist - Title" format
pub fn sanitize_track_filename(artist: &str, title: &str) -> String {
    let sanitized_artist = sanitize_filename(artist);
    let sanitized_title = sanitize_filename(title);
    format!("{} - {}", sanitized_artist, sanitized_title)
}

/// Creates a directory if it doesn't exist
pub async fn ensure_dir_exists(path: &std::path::Path) -> Result<()> {
    if !path.exists() {
        tokio::fs::create_dir_all(path).await?;
        info!("Created directory: {:?}", path);
    }
    Ok(())
}

// retry_with_backoff removed - not used

// format_file_size and format_duration removed - not used

/// Generates a unique ID for downloads
pub fn generate_download_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Encrypts sensitive data using AES-GCM
pub fn encrypt_data(data: &str, key: &[u8]) -> Result<String> {
    use aes_gcm::{Aes256Gcm, Key, Nonce, KeyInit};
    use aes_gcm::aead::Aead;
    
    let key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(b"unique nonce"); // In production, use a random nonce
    
    let ciphertext = cipher.encrypt(nonce, data.as_bytes())
        .map_err(|e| AppError::Encryption(format!("Encryption failed: {}", e)))?;
    
    Ok(hex::encode(ciphertext))
}

/// Decrypts sensitive data using AES-GCM
pub fn decrypt_data(encrypted_data: &str, key: &[u8]) -> Result<String> {
    use aes_gcm::{Aes256Gcm, Key, Nonce, KeyInit};
    use aes_gcm::aead::Aead;
    
    let key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(b"unique nonce"); // In production, use the same nonce used for encryption
    
    let ciphertext = hex::decode(encrypted_data)
        .map_err(|e| AppError::Encryption(format!("Hex decoding failed: {}", e)))?;
    
    let plaintext = cipher.decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| AppError::Encryption(format!("Decryption failed: {}", e)))?;
    
    String::from_utf8(plaintext)
        .map_err(|e| AppError::Encryption(format!("UTF-8 conversion failed: {}", e)))
}
