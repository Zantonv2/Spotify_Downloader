use crate::errors::Result;
use crate::metadata::{MetadataInfo, CoverArtInfo};
use reqwest;
use serde_json::Value;
use std::collections::HashMap;
use log;
use std::time::Duration;
use reqwest::Proxy;

pub struct MetadataProvider {
    client: reqwest::Client,
    api_keys: HashMap<String, String>,
    proxy_url: Option<String>,
}

impl MetadataProvider {
    pub fn new() -> Self {
        Self::new_with_proxy(None)
    }

    pub fn new_with_proxy(proxy_url: Option<String>) -> Self {
        // Create HTTP client with optimized timeouts for faster responses
        let mut builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))  // Reduced from 30s to 10s
            .connect_timeout(Duration::from_secs(5))  // Reduced from 10s to 5s
            .danger_accept_invalid_certs(false) // Keep security but handle cert issues
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36");

        // Add proxy if provided
        if let Some(proxy_url) = &proxy_url {
            log::info!("Using proxy for metadata requests: {}", proxy_url);
            if let Ok(proxy) = Proxy::all(proxy_url) {
                builder = builder.proxy(proxy);
            } else {
                log::warn!("Failed to configure proxy: {}", proxy_url);
            }
        }

        let client = builder
            .build()
            .unwrap_or_else(|_| {
                log::warn!("Failed to create HTTP client with proxy, falling back to basic client");
                reqwest::Client::new()
            });
        
        Self {
            client,
            api_keys: HashMap::new(),
            proxy_url,
        }
    }

    pub fn set_api_key(&mut self, service: &str, key: String) {
        self.api_keys.insert(service.to_string(), key);
    }

    pub fn set_proxy(&mut self, proxy_url: Option<String>) {
        self.proxy_url = proxy_url;
        // Recreate client with new proxy settings
        *self = Self::new_with_proxy(self.proxy_url.clone());
    }

    pub fn get_proxy(&self) -> Option<&String> {
        self.proxy_url.as_ref()
    }

    pub async fn search_metadata(&self, artist: &str, title: &str) -> Result<Option<MetadataInfo>> {
        // Run all three sources in parallel for comprehensive metadata collection
        let spotify_future = self.try_spotify(artist, title);
        let musicbrainz_future = self.try_musicbrainz(artist, title);
        let itunes_future = self.try_itunes(artist, title);
        
        // Wait for all to complete (or timeout)
        let (spotify_result, musicbrainz_result, itunes_result) = tokio::join!(
            spotify_future,
            musicbrainz_future,
            itunes_future
        );
        
        let spotify_metadata = spotify_result.ok().flatten();
        let musicbrainz_metadata = musicbrainz_result.ok().flatten();
        let itunes_metadata = itunes_result.ok().flatten();
        
        // Combine the metadata, prioritizing Spotify for basic info and MusicBrainz for details
        let mut combined_metadata = MetadataInfo {
            title: "Unknown Title".to_string(),
            artist: "Unknown Artist".to_string(),
            album: None,
            year: None,
            genre: None,
            track_number: None,
            disc_number: None,
            album_artist: None,
            composer: None,
            isrc: None,
            cover_art_url: None,
            lyrics: None,
        };
        
        // Combine metadata from all three sources for comprehensive coverage
        
        // Start with Spotify data (usually most accurate for basic info)
        if let Some(spotify) = spotify_metadata {
            combined_metadata.title = spotify.title;
            combined_metadata.artist = spotify.artist;
            combined_metadata.album = spotify.album;
            combined_metadata.cover_art_url = spotify.cover_art_url;
        }
        
        // Enhance with MusicBrainz data (best for detailed metadata)
        if let Some(musicbrainz) = musicbrainz_metadata {
            // Use MusicBrainz data to fill in missing or enhance existing data
            if combined_metadata.title == "Unknown Title" || combined_metadata.title.is_empty() {
                combined_metadata.title = musicbrainz.title;
            }
            if combined_metadata.artist == "Unknown Artist" || combined_metadata.artist.is_empty() {
                combined_metadata.artist = musicbrainz.artist;
            }
            if combined_metadata.album.is_none() || combined_metadata.album.as_ref().unwrap().is_empty() {
                combined_metadata.album = musicbrainz.album;
            }
            
            // MusicBrainz excels at detailed metadata
            combined_metadata.year = musicbrainz.year.or(combined_metadata.year);
            combined_metadata.genre = musicbrainz.genre.or(combined_metadata.genre);
            combined_metadata.disc_number = musicbrainz.disc_number.or(combined_metadata.disc_number);
            combined_metadata.album_artist = musicbrainz.album_artist.or(combined_metadata.album_artist);
            combined_metadata.composer = musicbrainz.composer.or(combined_metadata.composer);
            combined_metadata.isrc = musicbrainz.isrc.or(combined_metadata.isrc);
            
            // Use MusicBrainz cover art if Spotify didn't provide one
            if combined_metadata.cover_art_url.is_none() {
                combined_metadata.cover_art_url = musicbrainz.cover_art_url;
            }
        }
        
        // Fill remaining gaps with iTunes data
        if let Some(itunes) = itunes_metadata {
            // Fill in any remaining missing basic info
            if combined_metadata.title == "Unknown Title" || combined_metadata.title.is_empty() {
                combined_metadata.title = itunes.title;
            }
            if combined_metadata.artist == "Unknown Artist" || combined_metadata.artist.is_empty() {
                combined_metadata.artist = itunes.artist;
            }
            if combined_metadata.album.is_none() || combined_metadata.album.as_ref().unwrap().is_empty() {
                combined_metadata.album = itunes.album;
            }
            
            // Fill in missing detailed metadata
            combined_metadata.year = itunes.year.or(combined_metadata.year);
            combined_metadata.genre = itunes.genre.or(combined_metadata.genre);
            
            // Use iTunes cover art as final fallback
            if combined_metadata.cover_art_url.is_none() {
                combined_metadata.cover_art_url = itunes.cover_art_url;
            }
        }
        
        // If we got any useful data, return it
        if combined_metadata.title != "Unknown Title" || combined_metadata.artist != "Unknown Artist" {
            Ok(Some(combined_metadata))
        } else {
            Ok(None)
        }
    }

    pub async fn search_cover_art(&self, artist: &str, title: &str, album: Option<&str>) -> Result<Option<CoverArtInfo>> {
        // Try multiple cover art sources in order of preference
        let providers = vec![
            self.try_spotify_cover(artist, title, album).await,
            self.try_itunes_cover(artist, title, album).await,
        ];

        for result in providers {
            match result {
                Ok(Some(cover_art)) => return Ok(Some(cover_art)),
                Ok(None) => continue,
                Err(_) => continue,
            }
        }

        Ok(None)
    }

    async fn try_spotify(&self, artist: &str, title: &str) -> Result<Option<MetadataInfo>> {
        if let Some(access_token) = self.api_keys.get("spotify") {
            let query = format!("artist:{} track:{}", artist, title);
            let url = format!(
                "https://api.spotify.com/v1/search?q={}&type=track&limit=1",
                urlencoding::encode(&query)
            );

            let response = self.client
                .get(&url)
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await?;
            
            if response.status().is_success() {
                let json: Value = response.json().await?;
                if let Some(tracks) = json["tracks"]["items"].as_array() {
                    if let Some(track) = tracks.first() {
                        return Ok(Some(self.parse_spotify_track(track)));
                    }
                }
            }
        }

        Ok(None)
    }

    async fn try_musicbrainz(&self, artist: &str, title: &str) -> Result<Option<MetadataInfo>> {
        // Clean up the artist and title for better search results
        let clean_artist = self.clean_search_term(artist);
        let clean_title = self.clean_search_term(title);
        
        // Try complex query first with includes for more metadata
        let complex_query = format!("artist:{} AND recording:{}", clean_artist, clean_title);
        let complex_url = format!(
            "https://musicbrainz.org/ws/2/recording?query={}&fmt=json&limit=1&inc=releases+tags+artist-credits",
            urlencoding::encode(&complex_query)
        );

        log::debug!("Trying MusicBrainz complex search: {}", complex_url);

        match self.make_musicbrainz_request(&complex_url).await {
            Ok(Some(metadata)) => return Ok(Some(metadata)),
            Ok(None) => log::debug!("No results from complex query"),
            Err(e) => log::warn!("Complex query failed: {}", e),
        }

        // Try simpler query as fallback with includes for more metadata
        let simple_query = format!("{} {}", clean_artist, clean_title);
        let simple_url = format!(
            "https://musicbrainz.org/ws/2/recording?query={}&fmt=json&limit=1&inc=releases+tags+artist-credits",
            urlencoding::encode(&simple_query)
        );

        log::debug!("Trying MusicBrainz simple search: {}", simple_url);

        match self.make_musicbrainz_request(&simple_url).await {
            Ok(Some(metadata)) => {
                log::debug!("Found metadata with simple query");
                Ok(Some(metadata))
            }
            Ok(None) => {
                log::debug!("No results from simple query either");
                Ok(None)
            }
            Err(e) => {
                log::warn!("Simple query also failed: {}", e);
                Ok(None)
            }
        }
    }

    async fn make_musicbrainz_request(&self, url: &str) -> Result<Option<MetadataInfo>> {
        // Retry logic for MusicBrainz with exponential backoff
        let mut retries = 0;
        let max_retries = 2;
        
        loop {
            let response = self.client
                .get(url)
                .header("User-Agent", "SpotifyDownloader/1.0")
                .header("Accept", "application/json")
                .send()
                .await
                .map_err(|e| {
                    log::warn!("MusicBrainz request failed: {}", e);
                    e
                })?;
        
            log::debug!("MusicBrainz response status: {}", response.status());
            
            if response.status().is_success() {
                let json: Value = response.json().await.map_err(|e| {
                    log::warn!("Failed to parse MusicBrainz JSON: {}", e);
                    e
                })?;
                
                log::debug!("MusicBrainz response: {}", serde_json::to_string_pretty(&json).unwrap_or_default());
                
                if let Some(recordings) = json["recordings"].as_array() {
                    log::debug!("Found {} recordings", recordings.len());
                    if let Some(recording) = recordings.first() {
                        log::debug!("Using first recording: {}", serde_json::to_string_pretty(recording).unwrap_or_default());
                        return Ok(Some(self.parse_musicbrainz_recording(recording)));
                    }
                } else {
                    log::debug!("No recordings array in response");
                }
                return Ok(None);
            } else {
                let status = response.status();
                if status == 503 || status == 429 {
                    if retries < max_retries {
                        retries += 1;
                        let delay = std::time::Duration::from_millis(1000 * retries as u64);
                        log::warn!("MusicBrainz API error {} (attempt {}/{}), retrying in {:?}", status, retries, max_retries + 1, delay);
                        tokio::time::sleep(delay).await;
                        continue;
                    } else {
                        if status == 503 {
                            log::warn!("MusicBrainz API temporarily unavailable (503), skipping this source after {} retries", max_retries);
                        } else {
                            log::warn!("MusicBrainz API rate limited (429), skipping this source after {} retries", max_retries);
                        }
                        return Ok(None);
                    }
                } else {
                    log::warn!("MusicBrainz API error: {}", status);
                    return Ok(None);
                }
            }
        }
    }

    fn clean_search_term(&self, term: &str) -> String {
        // Remove common suffixes and clean up the search term
        term
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
            .trim()
            .to_string()
    }

    async fn try_deezer(&self, artist: &str, title: &str) -> Result<Option<MetadataInfo>> {
        if let Some(_api_key) = self.api_keys.get("deezer") {
            let query = format!("{} {}", artist, title);
            let url = format!(
                "https://api.deezer.com/search?q={}&limit=1",
                urlencoding::encode(&query)
            );

            let response = self.client.get(&url).send().await?;
            
            if response.status().is_success() {
                let json: Value = response.json().await?;
                if let Some(tracks) = json["data"].as_array() {
                    if let Some(track) = tracks.first() {
                        return Ok(Some(self.parse_deezer_track(track)));
                    }
                }
            }
        }

        Ok(None)
    }

    async fn try_spotify_cover(&self, artist: &str, title: &str, _album: Option<&str>) -> Result<Option<CoverArtInfo>> {
        if let Some(access_token) = self.api_keys.get("spotify") {
            let query = format!("artist:{} track:{}", artist, title);
            let url = format!(
                "https://api.spotify.com/v1/search?q={}&type=track&limit=1",
                urlencoding::encode(&query)
            );

            let response = self.client
                .get(&url)
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await?;
            
            if response.status().is_success() {
                let json: Value = response.json().await?;
                if let Some(tracks) = json["tracks"]["items"].as_array() {
                    if let Some(track) = tracks.first() {
                        if let Some(album) = track["album"].as_object() {
                            if let Some(images) = album["images"].as_array() {
                                if let Some(image) = images.first() {
                                    if let Some(url) = image["url"].as_str() {
                                        return Ok(Some(CoverArtInfo {
                                            url: url.to_string(),
                                            data: None,
                                            mime_type: Some("image/jpeg".to_string()),
                                        }));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    async fn try_itunes(&self, artist: &str, title: &str) -> Result<Option<MetadataInfo>> {
        let search_term = format!("{} {}", artist, title);
        let url = format!(
            "https://itunes.apple.com/search?term={}&media=music&limit=1",
            urlencoding::encode(&search_term)
        );

        log::debug!("Trying iTunes search: {}", url);

        let response = self.client
            .get(&url)
            .header("User-Agent", "SpotifyDownloader/1.0")
            .send()
            .await
            .map_err(|e| {
                log::warn!("iTunes request failed: {}", e);
                e
            })?;

        if response.status().is_success() {
            let json: Value = response.json().await.map_err(|e| {
                log::warn!("Failed to parse iTunes JSON: {}", e);
                e
            })?;

            if let Some(results) = json["results"].as_array() {
                if let Some(track) = results.first() {
                    return Ok(Some(self.parse_itunes_track(track)));
                }
            }
        } else {
            log::warn!("iTunes API error: {}", response.status());
        }

        Ok(None)
    }

    fn parse_itunes_track(&self, track: &Value) -> MetadataInfo {
        MetadataInfo {
            title: track["trackName"].as_str().unwrap_or("Unknown Title").to_string(),
            artist: track["artistName"].as_str().unwrap_or("Unknown Artist").to_string(),
            album: track["collectionName"].as_str().map(|s| s.to_string()),
            year: track["releaseDate"].as_str()
                .and_then(|date| date.split('-').next())
                .and_then(|year| year.parse::<u32>().ok()),
            genre: track["primaryGenreName"].as_str().map(|s| s.to_string()),
            track_number: None, // Will be set to queue position
            disc_number: None,
            album_artist: track["artistName"].as_str().map(|s| s.to_string()),
            composer: None,
            isrc: None,
            cover_art_url: track["artworkUrl100"].as_str()
                .map(|url| url.replace("100x100", "600x600")), // Get higher resolution
            lyrics: None,
        }
    }

    async fn try_itunes_cover(&self, artist: &str, title: &str, _album: Option<&str>) -> Result<Option<CoverArtInfo>> {
        let query = format!("{} {}", artist, title);
        let url = format!(
            "https://itunes.apple.com/search?term={}&media=music&limit=1",
            urlencoding::encode(&query)
        );

        let response = self.client.get(&url).send().await?;
        
        if response.status().is_success() {
            let json: Value = response.json().await?;
            if let Some(results) = json["results"].as_array() {
                if let Some(track) = results.first() {
                    if let Some(artwork_url) = track["artworkUrl100"].as_str() {
                        // Get higher resolution artwork
                        let high_res_url = artwork_url.replace("100x100", "600x600");
                        return Ok(Some(CoverArtInfo {
                            url: high_res_url,
                            data: None,
                            mime_type: Some("image/jpeg".to_string()),
                        }));
                    }
                }
            }
        }

        Ok(None)
    }

    fn parse_spotify_track(&self, track: &Value) -> MetadataInfo {
        // Sanitize strings to prevent JSON issues
        let sanitize_str = |s: Option<&str>| -> String {
            s.map(|s| {
                s.chars()
                    .filter(|c| !c.is_control() && *c != '\u{FFFD}')
                    .collect::<String>()
                    .trim()
                    .to_string()
            }).unwrap_or_else(|| "Unknown".to_string())
        };

        let title = sanitize_str(track["name"].as_str());
        let artists = track["artists"].as_array()
            .map(|arr| arr.iter()
                .filter_map(|artist| artist["name"].as_str())
                .map(|name| sanitize_str(Some(name)))
                .collect::<Vec<_>>()
                .join(", "))
            .unwrap_or_else(|| "Unknown Artist".to_string());
        
        let album = track["album"]["name"].as_str().map(|s| sanitize_str(Some(s)));
        let year = track["album"]["release_date"].as_str()
            .and_then(|date| date.split('-').next())
            .and_then(|year| year.parse::<u32>().ok());
        
        let cover_art_url = track["album"]["images"].as_array()
            .and_then(|images| images.first())
            .and_then(|image| image["url"].as_str())
            .map(|s| sanitize_str(Some(s)));

        let isrc = track["external_ids"]["isrc"].as_str().map(|s| sanitize_str(Some(s)));

        MetadataInfo {
            title,
            artist: artists,
            album,
            year,
            genre: None,
            track_number: None, // Track number is always set to queue position
            disc_number: track["disc_number"].as_u64().map(|n| n as u32),
            album_artist: track["album"]["artists"].as_array()
                .and_then(|arr| arr.first())
                .and_then(|artist| artist["name"].as_str())
                .map(|s| sanitize_str(Some(s))),
            composer: None,
            isrc,
            cover_art_url,
            lyrics: None,
        }
    }

    fn parse_musicbrainz_recording(&self, recording: &Value) -> MetadataInfo {
        // Sanitize strings to prevent JSON issues
        let sanitize_str = |s: Option<&str>| -> String {
            s.map(|s| {
                s.chars()
                    .filter(|c| !c.is_control() && *c != '\u{FFFD}')
                    .collect::<String>()
                    .trim()
                    .to_string()
            }).unwrap_or_else(|| "Unknown".to_string())
        };

        let title = sanitize_str(recording["title"].as_str());
        
        // Extract artists (main performers)
        let artists = recording["artist-credit"].as_array()
            .map(|arr| arr.iter()
                .filter_map(|credit| credit["name"].as_str())
                .map(|name| sanitize_str(Some(name)))
                .collect::<Vec<_>>()
                .join(", "))
            .unwrap_or_else(|| "Unknown Artist".to_string());

        // Extract album information from releases
        let (album, year, track_number, disc_number, album_artist) = recording["releases"].as_array()
            .and_then(|releases| releases.first())
            .map(|release| {
                let album = release["title"].as_str().map(|s| sanitize_str(Some(s)));
                let year = release["date"].as_str()
                    .and_then(|date| date.split('-').next())
                    .and_then(|year| year.parse::<u32>().ok());
                
                // Extract track number and disc number from media
                let (track_num, disc_num) = release["media"].as_array()
                    .and_then(|media| media.first())
                    .map(|medium| {
                        let disc_number = medium["position"].as_u64().map(|n| n as u32);
                        let track_number = medium["tracks"].as_array()
                            .and_then(|tracks| tracks.first())
                            .and_then(|track| track["position"].as_u64().map(|n| n as u32));
                        (track_number, disc_number)
                    })
                    .unwrap_or((None, None));
                
                // Extract album artist
                let album_artist = release["artist-credit"].as_array()
                    .map(|arr| arr.iter()
                        .filter_map(|credit| credit["name"].as_str())
                        .map(|name| sanitize_str(Some(name)))
                        .collect::<Vec<_>>()
                        .join(", "))
                    .unwrap_or_else(|| "Unknown Artist".to_string());
                
                (album, year, track_num, disc_num, Some(album_artist))
            })
            .unwrap_or((None, None, None, None, None));

        // Extract genre from tags
        let genre = recording["tags"].as_array()
            .and_then(|tags| tags.first())
            .and_then(|tag| tag["name"].as_str())
            .map(|s| sanitize_str(Some(s)));

        // Extract composer from artist-credits (look for composer role)
        let composer = recording["artist-credit"].as_array()
            .and_then(|arr| arr.iter()
                .find(|credit| {
                    credit["artist"]["type"].as_str() == Some("Person") &&
                    credit["artist"]["name"].as_str().is_some()
                })
                .and_then(|credit| credit["artist"]["name"].as_str())
                .map(|name| sanitize_str(Some(name)))
            );

        let isrc = recording["isrc-list"].as_array()
            .and_then(|isrcs| isrcs.first())
            .and_then(|isrc| isrc.as_str())
            .map(|s| sanitize_str(Some(s)));

        MetadataInfo {
            title,
            artist: artists,
            album,
            year,
            genre,
            track_number,
            disc_number,
            album_artist,
            composer,
            isrc,
            cover_art_url: None, // MusicBrainz doesn't provide cover art URLs
            lyrics: None,
        }
    }

    fn parse_deezer_track(&self, track: &Value) -> MetadataInfo {
        // Sanitize strings to prevent JSON issues
        let sanitize_str = |s: Option<&str>| -> String {
            s.map(|s| {
                s.chars()
                    .filter(|c| !c.is_control() && *c != '\u{FFFD}')
                    .collect::<String>()
                    .trim()
                    .to_string()
            }).unwrap_or_else(|| "Unknown".to_string())
        };

        let title = sanitize_str(track["title"].as_str());
        let artist = sanitize_str(track["artist"]["name"].as_str());
        let album = track["album"]["title"].as_str().map(|s| sanitize_str(Some(s)));
        let year = track["release_date"].as_str()
            .and_then(|date| date.split('-').next())
            .and_then(|year| year.parse::<u32>().ok());
        
        let cover_art_url = track["album"]["cover_medium"].as_str().map(|s| sanitize_str(Some(s)));

        MetadataInfo {
            title,
            artist,
            album,
            year,
            genre: track["genre"].as_str().map(|s| sanitize_str(Some(s))),
            track_number: None, // Track number is always set to queue position
            disc_number: track["disk_number"].as_u64().map(|n| n as u32),
            album_artist: track["album"]["artist"]["name"].as_str().map(|s| sanitize_str(Some(s))),
            composer: None,
            isrc: track["isrc"].as_str().map(|s| sanitize_str(Some(s))),
            cover_art_url,
            lyrics: None,
        }
    }
}
