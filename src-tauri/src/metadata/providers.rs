use crate::errors::Result;
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct MetadataProvider {
    client: Client,
    api_keys: HashMap<String, String>,
}

impl MetadataProvider {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .build()
                .unwrap_or_else(|_| Client::new()),
            api_keys: HashMap::new(),
        }
    }

    pub fn new_with_proxy(proxy_url: Option<String>) -> Self {
        let mut builder = Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36");

        if let Some(proxy_url) = proxy_url {
            if let Ok(proxy) = reqwest::Proxy::all(&proxy_url) {
                builder = builder.proxy(proxy);
            }
        }
        
        Self {
            client: builder.build().unwrap_or_else(|_| Client::new()),
            api_keys: HashMap::new(),
        }
    }

    pub fn set_api_key(&mut self, service: &str, key: String) {
        self.api_keys.insert(service.to_string(), key);
    }

    pub async fn search_metadata(&self, artist: &str, title: &str) -> Result<Option<crate::metadata::MetadataInfo>> {
        self.search_metadata_with_album(artist, title, None).await
    }

    pub async fn search_metadata_with_album(&self, artist: &str, title: &str, album: Option<&str>) -> Result<Option<crate::metadata::MetadataInfo>> {
        // Try Spotify first (most reliable)
        if let Some(mut spotify_metadata) = self.try_spotify(artist, title, album).await? {
            // Ensure cover art is included
            if spotify_metadata.cover_art_url.is_none() {
                if let Some(cover_art) = self.try_spotify_cover(artist, title, album).await? {
                    spotify_metadata.cover_art_url = Some(cover_art.url);
                }
            }
            return Ok(Some(spotify_metadata));
        }

        // Try iTunes as fallback
        if let Some(mut itunes_metadata) = self.try_itunes(artist, title).await? {
            // Ensure cover art is included
            if itunes_metadata.cover_art_url.is_none() {
                if let Some(cover_art) = self.try_itunes_cover(artist, title, album).await? {
                    itunes_metadata.cover_art_url = Some(cover_art.url);
                }
            }
            return Ok(Some(itunes_metadata));
        }

        // Try Cover Art Archive as final fallback
        if let Some(mut cover_art_metadata) = self.try_cover_art_archive(artist, title, album).await? {
            // Cover Art Archive provides cover art directly
            return Ok(Some(cover_art_metadata));
        }

        Ok(None)
    }

    async fn try_cover_art_archive(&self, artist: &str, title: &str, album: Option<&str>) -> Result<Option<crate::metadata::MetadataInfo>> {
        // First, search MusicBrainz for the release ID
        let query = if let Some(album_name) = album {
            format!("artist:\"{}\" AND recording:\"{}\" AND release:\"{}\"", artist, title, album_name)
        } else {
            format!("artist:\"{}\" AND recording:\"{}\"", artist, title)
        };

        let url = format!(
            "https://musicbrainz.org/ws/2/recording?query={}&fmt=json&limit=5",
            urlencoding::encode(&query)
        );

        let response = self.client
            .get(&url)
            .header("User-Agent", "SpotifyDownloader/1.0 (contact@example.com)")
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(None);
        }

        let json: Value = response.json().await?;
        let recordings = match json["recordings"].as_array() {
            Some(recordings) if !recordings.is_empty() => recordings,
            _ => return Ok(None),
        };

        // Find the best match and get release ID
        for recording in recordings {
            let recording_title = recording["title"].as_str().unwrap_or("");
            let recording_artists: Vec<&str> = recording["artist-credit"]
                .as_array()
                .map(|arr| arr.iter()
                    .filter_map(|artist| artist["name"].as_str())
                    .collect())
                .unwrap_or_default();

            if self.is_good_match(artist, title, &recording_artists, recording_title) {
                // Get the first release ID
                if let Some(release_id) = recording["releases"]
                    .as_array()
                    .and_then(|releases| releases.first())
                    .and_then(|release| release["id"].as_str()) {
                    
                    // Try to get cover art from Cover Art Archive
                    if let Some(cover_art_url) = self.get_cover_art_from_archive(release_id).await? {
                        let metadata = crate::metadata::MetadataInfo {
                            title: recording_title.to_string(),
                            artist: recording_artists.join(", "),
                            album: recording["releases"]
                                .as_array()
                                .and_then(|releases| releases.first())
                                .and_then(|release| release["title"].as_str())
                                .map(|s| s.to_string()),
                            year: recording["releases"]
                                .as_array()
                                .and_then(|releases| releases.first())
                                .and_then(|release| release["date"].as_str())
                                .and_then(|date| date.split('-').next())
                                .and_then(|year| year.parse::<u32>().ok()),
                            genre: None,
                            track_number: None,
                            disc_number: None,
                            album_artist: None,
                            composer: None,
                            isrc: recording["isrcs"]
                                .as_array()
                                .and_then(|isrcs| isrcs.first())
                                .and_then(|isrc| isrc.as_str())
                                .map(|s| s.to_string()),
                            cover_art_url: Some(cover_art_url),
                            lyrics: None,
                        };

                        return Ok(Some(metadata));
                    }
                }
            }
        }

        Ok(None)
    }

    async fn get_cover_art_from_archive(&self, release_id: &str) -> Result<Option<String>> {
        // Use Cover Art Archive API to get cover art
        let url = format!("https://coverartarchive.org/release/{}/front", release_id);
        
        let response = self.client
            .get(&url)
            .send()
            .await?;

        if response.status().is_success() {
            // Get the redirect URL which contains the actual image
            if let Some(location) = response.headers().get("location") {
                if let Ok(cover_url) = location.to_str() {
                    return Ok(Some(cover_url.to_string()));
                }
            }
        }

        Ok(None)
    }

    async fn try_spotify(&self, artist: &str, title: &str, album: Option<&str>) -> Result<Option<crate::metadata::MetadataInfo>> {
        let access_token = match self.api_keys.get("spotify") {
            Some(token) => token,
            None => return Ok(None),
        };

        // Build search query
        let query = if let Some(album_name) = album {
            format!("artist:\"{}\" track:\"{}\" album:\"{}\"", artist, title, album_name)
        } else {
            format!("artist:\"{}\" track:\"{}\"", artist, title)
        };

            let url = format!(
            "https://api.spotify.com/v1/search?q={}&type=track&limit=5",
                urlencoding::encode(&query)
            );

            let response = self.client
                .get(&url)
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await?;
            
        if !response.status().is_success() {
            return Ok(None);
        }

                let json: Value = response.json().await?;
        let tracks = match json["tracks"]["items"].as_array() {
            Some(tracks) if !tracks.is_empty() => tracks,
            _ => return Ok(None),
        };

        // Find the best match
        for track in tracks {
            let track_name = track["name"].as_str().unwrap_or("");
            let track_artists: Vec<&str> = track["artists"].as_array()
                .map(|arr| arr.iter()
                    .filter_map(|artist| artist["name"].as_str())
                    .collect())
                .unwrap_or_default();

            // Check if this is a good match
            if self.is_good_match(artist, title, &track_artists, track_name) {
                        return Ok(Some(self.parse_spotify_track(track)));
            }
        }

        Ok(None)
    }

    async fn try_itunes(&self, artist: &str, title: &str) -> Result<Option<crate::metadata::MetadataInfo>> {
        let query = format!("{} {}", artist, title);
        let url = format!(
            "https://itunes.apple.com/search?term={}&media=music&entity=song&limit=5",
            urlencoding::encode(&query)
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Ok(None);
        }

        let json: Value = response.json().await?;
        let results = match json["results"].as_array() {
            Some(results) if !results.is_empty() => results,
            _ => return Ok(None),
        };

        // Find the best match
        for result in results {
            let track_name = result["trackName"].as_str().unwrap_or("");
            let artist_name = result["artistName"].as_str().unwrap_or("");

            // Check if this is a good match
            if self.is_good_match(artist, title, &[artist_name], track_name) {
                return Ok(Some(self.parse_itunes_track(result)));
            }
        }

                Ok(None)
            }

    fn is_good_match(&self, target_artist: &str, target_title: &str, result_artists: &[&str], result_title: &str) -> bool {
        let target_artist_lower = target_artist.to_lowercase();
        let target_title_lower = target_title.to_lowercase();
        let result_title_lower = result_title.to_lowercase();

        // Check title similarity (most important)
        let title_similarity = self.calculate_similarity(&target_title_lower, &result_title_lower);
        if title_similarity < 0.6 {
            return false;
        }

        // Check artist similarity
        let mut best_artist_similarity = 0.0;
        for result_artist in result_artists {
            let result_artist_lower = result_artist.to_lowercase();
            let similarity = self.calculate_similarity(&target_artist_lower, &result_artist_lower);
            if similarity > best_artist_similarity {
                best_artist_similarity = similarity;
            }
        }

        // Both title and artist need to be reasonably similar
        title_similarity >= 0.6 && best_artist_similarity >= 0.5
    }

    fn calculate_similarity(&self, s1: &str, s2: &str) -> f32 {
        let words1: Vec<&str> = s1.split_whitespace().collect();
        let words2: Vec<&str> = s2.split_whitespace().collect();
        
        if words1.is_empty() || words2.is_empty() {
            return 0.0;
        }

        let matching_words = words1.iter()
            .filter(|word| words2.contains(word))
            .count();

        matching_words as f32 / words1.len() as f32
    }

    fn parse_spotify_track(&self, track: &Value) -> crate::metadata::MetadataInfo {
        let title = track["name"].as_str().unwrap_or("Unknown Title").to_string();
        let artists: Vec<String> = track["artists"].as_array()
            .map(|arr| arr.iter()
                .filter_map(|artist| artist["name"].as_str())
                .map(|s| s.to_string())
                .collect())
            .unwrap_or_default();
        let artist = artists.join(", ");

        let album = track["album"]["name"].as_str().map(|s| s.to_string());
        let year = track["album"]["release_date"].as_str()
            .and_then(|date| date.split('-').next())
            .and_then(|year| year.parse::<u32>().ok());

        let cover_art_url = track["album"]["images"].as_array()
            .and_then(|images| images.first())
            .and_then(|image| image["url"].as_str())
            .map(|s| s.to_string());

        let isrc = track["external_ids"]["isrc"].as_str().map(|s| s.to_string());

        crate::metadata::MetadataInfo {
            title,
            artist,
            album,
            year,
            genre: None,
            track_number: track["track_number"].as_u64().map(|n| n as u32),
            disc_number: track["disc_number"].as_u64().map(|n| n as u32),
            album_artist: track["album"]["artists"].as_array()
                .and_then(|arr| arr.first())
                .and_then(|artist| artist["name"].as_str())
                .map(|s| s.to_string()),
            composer: None,
            isrc,
            cover_art_url,
            lyrics: None,
        }
    }

    fn parse_itunes_track(&self, track: &Value) -> crate::metadata::MetadataInfo {
        let title = track["trackName"].as_str().unwrap_or("Unknown Title").to_string();
        let artist = track["artistName"].as_str().unwrap_or("Unknown Artist").to_string();
        let album = track["collectionName"].as_str().map(|s| s.to_string());
        let year = track["releaseDate"].as_str()
            .and_then(|date| date.split('-').next())
            .and_then(|year| year.parse::<u32>().ok());

        let cover_art_url = track["artworkUrl100"].as_str()
            .map(|url| url.replace("100x100", "600x600"));

        let genre = track["primaryGenreName"].as_str().map(|s| s.to_string());

        crate::metadata::MetadataInfo {
            title,
            artist,
            album,
            year,
            genre,
            track_number: track["trackNumber"].as_u64().map(|n| n as u32),
            disc_number: None,
            album_artist: None,
            composer: None,
            isrc: None,
            cover_art_url,
            lyrics: None,
        }
    }

    pub async fn search_cover_art(&self, artist: &str, title: &str, album: Option<&str>) -> Result<Option<crate::metadata::CoverArtInfo>> {
        log::info!("🖼️ [COVER] Searching for cover art: {} - {}", artist, title);
        
        // Try Spotify first
        if let Some(cover_art) = self.try_spotify_cover(artist, title, album).await? {
            log::info!("✅ [COVER] Found cover art via Spotify: {}", cover_art.url);
            return Ok(Some(cover_art));
        }

        // Try iTunes as fallback
        if let Some(cover_art) = self.try_itunes_cover(artist, title, album).await? {
            log::info!("✅ [COVER] Found cover art via iTunes: {}", cover_art.url);
            return Ok(Some(cover_art));
        }

        // Try album-specific searches if album is available
        if let Some(album_name) = album {
            if let Some(cover_art) = self.try_spotify_cover_by_album(artist, album_name).await? {
                log::info!("✅ [COVER] Found cover art via Spotify Album: {}", cover_art.url);
                return Ok(Some(cover_art));
            }

            if let Some(cover_art) = self.try_itunes_cover_by_album(artist, album_name).await? {
                log::info!("✅ [COVER] Found cover art via iTunes Album: {}", cover_art.url);
                return Ok(Some(cover_art));
            }
        }

        // Try Cover Art Archive as final fallback
        if let Some(cover_art) = self.try_cover_art_archive_cover(artist, title, album).await? {
            log::info!("✅ [COVER] Found cover art via Cover Art Archive: {}", cover_art.url);
            return Ok(Some(cover_art));
        }

        log::warn!("❌ [COVER] No cover art found for {} - {}", artist, title);
        Ok(None)
    }

    async fn try_spotify_cover(&self, artist: &str, title: &str, album: Option<&str>) -> Result<Option<crate::metadata::CoverArtInfo>> {
        let access_token = match self.api_keys.get("spotify") {
            Some(token) => token,
            None => {
                log::warn!("⚠️ [COVER] No Spotify API key available");
                return Ok(None);
            },
        };

        // Try multiple search strategies for better cover art results
        let search_queries = vec![
            // Strategy 1: Most specific - Artist + Title + Album (if available)
            if album.is_some() {
                format!("artist:\"{}\" track:\"{}\" album:\"{}\"", artist, title, album.unwrap())
            } else {
                format!("artist:\"{}\" track:\"{}\"", artist, title)
            },
            // Strategy 2: Exact match with quotes
            format!("\"{}\" \"{}\"", artist, title),
            // Strategy 3: Artist field with flexible title
            format!("artist:\"{}\" {}", artist, title),
            // Strategy 4: Simple concatenation
            format!("{} {}", artist, title),
        ];

        for (i, query) in search_queries.iter().enumerate() {
            log::info!("🖼️ [COVER] Spotify search strategy {}: {}", i + 1, query);
            let url = format!(
                "https://api.spotify.com/v1/search?q={}&type=track&limit=5",
                urlencoding::encode(query)
            );

            let response = self.client
                .get(&url)
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await?;
            
            if !response.status().is_success() {
                log::warn!("⚠️ [COVER] Spotify API returned status: {} for query: {}", response.status(), query);
                continue;
            }

                let json: Value = response.json().await?;
            let tracks = match json["tracks"]["items"].as_array() {
                Some(tracks) if !tracks.is_empty() => {
                    log::info!("🖼️ [COVER] Spotify found {} tracks for query: {}", tracks.len(), query);
                    tracks
                },
                _ => {
                    log::warn!("⚠️ [COVER] Spotify returned no tracks for query: {}", query);
                    continue;
                },
            };

            // Find the best match and extract cover art
            for track in tracks {
                let track_name = track["name"].as_str().unwrap_or("");
                let track_artists: Vec<&str> = track["artists"].as_array()
                    .map(|arr| arr.iter()
                        .filter_map(|artist| artist["name"].as_str())
                        .collect())
                    .unwrap_or_default();

                log::info!("🖼️ [COVER] Checking Spotify track: '{}' by '{}'", track_name, track_artists.join(", "));

                // Check if this is a good match
                if self.is_good_match(artist, title, &track_artists, track_name) {
                    log::info!("✅ [COVER] Spotify match found: '{}' by '{}'", track_name, track_artists.join(", "));
                        if let Some(album) = track["album"].as_object() {
                            if let Some(images) = album["images"].as_array() {
                                log::info!("🖼️ [COVER] Found {} album images", images.len());
                                if let Some(largest_image) = images.iter()
                                    .max_by_key(|img| img["width"].as_u64().unwrap_or(0)) {
                                    if let Some(cover_url) = largest_image["url"].as_str() {
                                        log::info!("✅ [COVER] Spotify cover art URL: {}", cover_url);
                                        
                                        // Download the cover art data immediately
                                        match self.download_cover_art_data(cover_url).await {
                                            Ok(cover_data) => {
                                                log::info!("✅ [COVER] Downloaded Spotify cover art: {} bytes", cover_data.data.len());
                                                return Ok(Some(crate::metadata::CoverArtInfo {
                                                    url: cover_url.to_string(),
                                                    data: Some(cover_data.data),
                                                    mime_type: Some(cover_data.mime_type),
                                                }));
                                            }
                                            Err(e) => {
                                                log::warn!("⚠️ [COVER] Failed to download Spotify cover art: {}", e);
                                                // Return URL-only version as fallback
                                                return Ok(Some(crate::metadata::CoverArtInfo {
                                                    url: cover_url.to_string(),
                                                    data: None,
                                                    mime_type: Some("image/jpeg".to_string()),
                                                }));
                                            }
                                        }
                                    } else {
                                        log::warn!("⚠️ [COVER] Spotify match found but no cover art URL");
                                    }
                                } else {
                                    log::warn!("⚠️ [COVER] Spotify match found but no album images");
                                }
                            } else {
                                log::warn!("⚠️ [COVER] Spotify match found but no album object");
                            }
                        } else {
                            log::warn!("⚠️ [COVER] Spotify match found but no album");
                        }
                }
            }
        }

        Ok(None)
    }

    async fn try_itunes_cover(&self, artist: &str, title: &str, _album: Option<&str>) -> Result<Option<crate::metadata::CoverArtInfo>> {
        let query = format!("{} {}", artist, title);
        let url = format!(
            "https://itunes.apple.com/search?term={}&media=music&entity=song&limit=5",
            urlencoding::encode(&query)
        );

        log::info!("🖼️ [COVER] Trying iTunes search: {}", query);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            log::warn!("⚠️ [COVER] iTunes API returned status: {}", response.status());
            return Ok(None);
        }

        let json: Value = response.json().await?;
        let results = match json["results"].as_array() {
            Some(results) if !results.is_empty() => {
                log::info!("🖼️ [COVER] iTunes found {} results", results.len());
                results
            },
            _ => {
                log::warn!("⚠️ [COVER] iTunes returned no results");
                return Ok(None);
            },
        };

        // Find the best match and extract cover art
        for result in results {
            let track_name = result["trackName"].as_str().unwrap_or("");
            let artist_name = result["artistName"].as_str().unwrap_or("");

            log::info!("🖼️ [COVER] Checking iTunes result: '{}' by '{}'", track_name, artist_name);

            // Check if this is a good match
            if self.is_good_match(artist, title, &[artist_name], track_name) {
                log::info!("✅ [COVER] iTunes match found: '{}' by '{}'", track_name, artist_name);
                if let Some(cover_url) = result["artworkUrl100"].as_str() {
                    // Replace 100x100 with 600x600 for better quality
                    let high_res_url = cover_url.replace("100x100", "600x600");
                    log::info!("🖼️ [COVER] iTunes cover art URL: {}", high_res_url);
                    
                    // Download the cover art data immediately
                    match self.download_cover_art_data(&high_res_url).await {
                        Ok(cover_data) => {
                            log::info!("✅ [COVER] Downloaded iTunes cover art: {} bytes", cover_data.data.len());
                            return Ok(Some(crate::metadata::CoverArtInfo {
                                url: high_res_url,
                                data: Some(cover_data.data),
                                mime_type: Some(cover_data.mime_type),
                            }));
                        }
                        Err(e) => {
                            log::warn!("⚠️ [COVER] Failed to download iTunes cover art: {}", e);
                            // Return URL-only version as fallback
                            return Ok(Some(crate::metadata::CoverArtInfo {
                                url: high_res_url,
                                data: None,
                                mime_type: Some("image/jpeg".to_string()),
                            }));
                        }
                    }
                } else {
                    log::warn!("⚠️ [COVER] iTunes match found but no cover art URL");
                }
            }
        }

        Ok(None)
    }

    async fn download_cover_art_data(&self, url: &str) -> Result<crate::metadata::CoverArtData> {
        log::info!("🖼️ [DOWNLOAD] Downloading cover art from: {}", url);
        
        let response = self.client
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
        
        Ok(crate::metadata::CoverArtData {
            data: data.to_vec(),
            mime_type,
        })
    }

    async fn try_cover_art_archive_cover(&self, artist: &str, title: &str, album: Option<&str>) -> Result<Option<crate::metadata::CoverArtInfo>> {
        // First, search MusicBrainz for the release ID
        let query = if let Some(album_name) = album {
            format!("artist:\"{}\" AND recording:\"{}\" AND release:\"{}\"", artist, title, album_name)
        } else {
            format!("artist:\"{}\" AND recording:\"{}\"", artist, title)
        };

        let url = format!(
            "https://musicbrainz.org/ws/2/recording?query={}&fmt=json&limit=5",
            urlencoding::encode(&query)
        );

        let response = self.client
            .get(&url)
            .header("User-Agent", "SpotifyDownloader/1.0 (contact@example.com)")
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(None);
        }

        let json: Value = response.json().await?;
        let recordings = match json["recordings"].as_array() {
            Some(recordings) if !recordings.is_empty() => recordings,
            _ => return Ok(None),
        };

        // Find the best match and get release ID
        for recording in recordings {
            let recording_title = recording["title"].as_str().unwrap_or("");
            let recording_artists: Vec<&str> = recording["artist-credit"]
                .as_array()
                .map(|arr| arr.iter()
                    .filter_map(|artist| artist["name"].as_str())
                    .collect())
                .unwrap_or_default();

            if self.is_good_match(artist, title, &recording_artists, recording_title) {
                // Get the first release ID
                if let Some(release_id) = recording["releases"]
                    .as_array()
                    .and_then(|releases| releases.first())
                    .and_then(|release| release["id"].as_str()) {
                    
                    // Try to get cover art from Cover Art Archive
                    if let Some(cover_art_url) = self.get_cover_art_from_archive(release_id).await? {
                        return Ok(Some(crate::metadata::CoverArtInfo {
                            url: cover_art_url,
                            data: None,
                            mime_type: Some("image/jpeg".to_string()),
                        }));
                    }
                }
            }
        }

        Ok(None)
    }

    async fn try_spotify_cover_by_album(&self, artist: &str, album: &str) -> Result<Option<crate::metadata::CoverArtInfo>> {
        let access_token = match self.api_keys.get("spotify") {
            Some(token) => token,
            None => return Ok(None),
        };

        let query = format!("artist:\"{}\" album:\"{}\"", artist, album);
        let url = format!(
            "https://api.spotify.com/v1/search?q={}&type=album&limit=5",
            urlencoding::encode(&query)
        );

        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(None);
        }

            let json: Value = response.json().await?;
        let albums = match json["albums"]["items"].as_array() {
            Some(albums) if !albums.is_empty() => albums,
            _ => return Ok(None),
        };

        // Find the best album match
        for album_obj in albums {
            let album_name = album_obj["name"].as_str().unwrap_or("");
            let album_artists: Vec<&str> = album_obj["artists"]
                .as_array()
                .map(|arr| arr.iter()
                    .filter_map(|artist| artist["name"].as_str())
                    .collect())
                .unwrap_or_default();

            if self.is_good_match(artist, album, &album_artists, album_name) {
                if let Some(images) = album_obj["images"].as_array() {
                    if let Some(largest_image) = images.iter()
                        .max_by_key(|img| img["width"].as_u64().unwrap_or(0)) {
                        if let Some(cover_url) = largest_image["url"].as_str() {
                            return Ok(Some(crate::metadata::CoverArtInfo {
                                url: cover_url.to_string(),
                            data: None,
                            mime_type: Some("image/jpeg".to_string()),
                        }));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    async fn try_itunes_cover_by_album(&self, artist: &str, album: &str) -> Result<Option<crate::metadata::CoverArtInfo>> {
        let query = format!("{} {}", artist, album);
        let url = format!(
            "https://itunes.apple.com/search?term={}&media=music&entity=album&limit=5",
            urlencoding::encode(&query)
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Ok(None);
        }

        let json: Value = response.json().await?;
        let results = match json["results"].as_array() {
            Some(results) if !results.is_empty() => results,
            _ => return Ok(None),
        };

        // Find the best album match
        for result in results {
            let album_name = result["collectionName"].as_str().unwrap_or("");
            let artist_name = result["artistName"].as_str().unwrap_or("");

            if self.is_good_match(artist, album, &[artist_name], album_name) {
                if let Some(cover_url) = result["artworkUrl100"].as_str() {
                    let high_res_url = cover_url.replace("100x100", "600x600");
                    return Ok(Some(crate::metadata::CoverArtInfo {
                        url: high_res_url,
                        data: None,
                        mime_type: Some("image/jpeg".to_string()),
                    }));
                }
            }
        }

        Ok(None)
    }
}
