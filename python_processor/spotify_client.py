#!/usr/bin/env python3
"""
Spotify client using Spotipy for playlist and album operations
"""

import sys
import json
import spotipy
from spotipy.oauth2 import SpotifyClientCredentials
from typing import Dict, List, Any, Optional
import re
import requests

class SpotifyClient:
    def __init__(self, client_id: str, client_secret: str, proxy: Optional[str] = None):
        """Initialize Spotify client with credentials and optional proxy"""
        self.client_id = client_id
        self.client_secret = client_secret
        self.proxy = proxy
        
        # Set up authentication
        client_credentials_manager = SpotifyClientCredentials(
            client_id=client_id,
            client_secret=client_secret
        )
        
        # Configure Spotipy with proxy if provided
        if proxy:
            # Set up session with proxy
            session = requests.Session()
            session.proxies = {
                'http': proxy,
                'https': proxy
            }
            self.sp = spotipy.Spotify(
                client_credentials_manager=client_credentials_manager,
                requests_session=session
            )
        else:
            self.sp = spotipy.Spotify(client_credentials_manager=client_credentials_manager)
    
    def parse_spotify_url(self, url: str) -> tuple[str, str]:
        """Parse Spotify URL to extract type and ID"""
        # Remove query parameters
        url = url.split('?')[0]
        
        # Extract ID from different URL formats
        patterns = [
            r'spotify\.com/(track|album|playlist)/([a-zA-Z0-9]+)',
            r'spotify:track:([a-zA-Z0-9]+)',
            r'spotify:album:([a-zA-Z0-9]+)',
            r'spotify:playlist:([a-zA-Z0-9]+)'
        ]
        
        for pattern in patterns:
            match = re.search(pattern, url)
            if match:
                if 'spotify:' in pattern:
                    # URI format
                    return match.group(1), match.group(1)
                else:
                    # URL format
                    return match.group(1), match.group(2)
        
        raise ValueError(f"Invalid Spotify URL: {url}")
    
    def get_track(self, track_id: str) -> Dict[str, Any]:
        """Get track information"""
        try:
            track = self.sp.track(track_id)
            return self._format_track(track)
        except Exception as e:
            raise Exception(f"Failed to get track: {e}")
    
    def get_album(self, album_id: str) -> Dict[str, Any]:
        """Get album information with all tracks"""
        try:
            album = self.sp.album(album_id)
            tracks = self.sp.album_tracks(album_id, limit=50)
            
            # Get all tracks (handle pagination)
            all_tracks = []
            while tracks:
                all_tracks.extend(tracks['items'])
                tracks = self.sp.next(tracks) if tracks['next'] else None
            
            return {
                "type": "album",
                "data": self._format_album(album),
                "tracks": [self._format_track(track) for track in all_tracks]
            }
        except Exception as e:
            raise Exception(f"Failed to get album: {e}")
    
    def get_playlist(self, playlist_id: str) -> Dict[str, Any]:
        """Get playlist information with all tracks"""
        try:
            playlist = self.sp.playlist(playlist_id)
            tracks = self.sp.playlist_tracks(playlist_id, limit=100)
            
            # Get all tracks (handle pagination)
            all_tracks = []
            while tracks:
                for item in tracks['items']:
                    if item['track']:  # Skip None tracks
                        all_tracks.append(item['track'])
                tracks = self.sp.next(tracks) if tracks['next'] else None
            
            return {
                "type": "playlist",
                "data": self._format_playlist(playlist),
                "tracks": [self._format_track(track) for track in all_tracks]
            }
        except Exception as e:
            raise Exception(f"Failed to get playlist: {e}")
    
    def _format_track(self, track: Dict[str, Any]) -> Dict[str, Any]:
        """Format track data for our application"""
        artists = [artist['name'] for artist in track.get('artists', [])]
        artist = ', '.join(artists) if artists else 'Unknown Artist'
        
        return {
            "id": track.get('id', ''),
            "title": track.get('name', 'Unknown Title'),
            "artist": artist,
            "artists": [{"name": artist} for artist in artists],
            "album": track.get('album', {}).get('name', 'Unknown Album'),
            "duration": track.get('duration_ms', 0) // 1000,  # Convert to seconds
            "duration_ms": track.get('duration_ms', 0),
            "external_urls": track.get('external_urls', {}),
            "preview_url": track.get('preview_url'),
            "popularity": track.get('popularity', 0),
            "explicit": track.get('explicit', False),
            "track_number": track.get('track_number', 0),
            "disc_number": track.get('disc_number', 1)
        }
    
    def _format_album(self, album: Dict[str, Any]) -> Dict[str, Any]:
        """Format album data"""
        artists = [artist['name'] for artist in album.get('artists', [])]
        artist = ', '.join(artists) if artists else 'Unknown Artist'
        
        return {
            "id": album.get('id', ''),
            "name": album.get('name', 'Unknown Album'),
            "artist": artist,
            "artists": [{"name": artist} for artist in artists],
            "release_date": album.get('release_date', ''),
            "total_tracks": album.get('total_tracks', 0),
            "images": album.get('images', []),
            "external_urls": album.get('external_urls', {}),
            "album_type": album.get('album_type', 'album'),
            "popularity": album.get('popularity', 0)
        }
    
    def _format_playlist(self, playlist: Dict[str, Any]) -> Dict[str, Any]:
        """Format playlist data"""
        return {
            "id": playlist.get('id', ''),
            "name": playlist.get('name', 'Unknown Playlist'),
            "description": playlist.get('description', ''),
            "owner": playlist.get('owner', {}).get('display_name', 'Unknown'),
            "tracks": {
                "total": playlist.get('tracks', {}).get('total', 0)
            },
            "images": playlist.get('images', []),
            "external_urls": playlist.get('external_urls', {}),
            "public": playlist.get('public', False),
            "collaborative": playlist.get('collaborative', False)
        }

def main():
    """Main function for command line usage"""
    try:
        # Read JSON input from stdin
        input_data = json.loads(sys.stdin.read())
        
        client_id = input_data["client_id"]
        client_secret = input_data["client_secret"]
        url = input_data["url"]
        proxy = input_data.get("proxy")  # Optional proxy setting
        
        client = SpotifyClient(client_id, client_secret, proxy)
        url_type, spotify_id = client.parse_spotify_url(url)
        
        if url_type == 'track':
            result = client.get_track(spotify_id)
        elif url_type == 'album':
            result = client.get_album(spotify_id)
        elif url_type == 'playlist':
            result = client.get_playlist(spotify_id)
        else:
            raise ValueError(f"Unsupported URL type: {url_type}")
        
        print(json.dumps(result))
        
    except Exception as e:
        print(json.dumps({"error": str(e)}))
        sys.exit(1)

if __name__ == "__main__":
    main()
