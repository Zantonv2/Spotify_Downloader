#!/usr/bin/env python3
"""
Audio processing subprocess for Spotify Downloader
Handles downloading, transcoding, and metadata embedding using yt-dlp, FFmpeg, and mutagen
"""

import json
import sys
import os
import subprocess
import tempfile
import shutil
import uuid
import re
from pathlib import Path
from typing import Dict, Any, Optional, List
import logging
import ffmpeg

# Fuzzy matching for search results
try:
    from fuzzywuzzy import fuzz, process
    from fuzzywuzzy.utils import full_process
    FUZZY_AVAILABLE = True
except ImportError:
    FUZZY_AVAILABLE = False
    print("Warning: fuzzywuzzy not available - search ranking will be basic")

# Audio metadata handling
try:
    import mutagen
    from mutagen.mp4 import MP4
    from mutagen.flac import FLAC
    MUTAGEN_AVAILABLE = True
except ImportError:
    MUTAGEN_AVAILABLE = False
    logger.warning("mutagen not available - metadata embedding will be disabled")

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

class AudioProcessor:
    def __init__(self):
        self.temp_dir = Path(tempfile.gettempdir()) / "spotify_downloader"
        self.temp_dir.mkdir(exist_ok=True)
        self._ffmpeg_path = None  # Cache FFmpeg path
        
    def process_request(self, request: Dict[str, Any]) -> Dict[str, Any]:
        """Main entry point for processing requests"""
        try:
            action = request.get("action")
            
            if action == "download":
                return self.download_audio(request)
            elif action == "embed_metadata_only":
                return self.embed_metadata_only(request)
            elif action == "transcode":
                return self.transcode_audio(request)
            elif action == "embed_metadata":
                return self.embed_metadata(request)
            elif action == "embed_cover_art":
                return self.embed_cover_art(request)
            elif action == "embed_lyrics":
                return self.embed_lyrics(request)
            elif action == "validate_flac_metadata":
                return self.validate_flac_metadata(Path(request["file_path"]))
            elif action == "validate_wav_metadata":
                return self.validate_wav_metadata(Path(request["file_path"]))
            elif action == "validate_ogg_metadata":
                return self.validate_ogg_metadata(Path(request["file_path"]))
            elif action == "validate_opus_metadata":
                return self.validate_opus_metadata(Path(request["file_path"]))
            elif action == "validate_ape_metadata":
                return self.validate_ape_metadata(Path(request["file_path"]))
            elif action == "read_metadata":
                return self.read_metadata(request)
            elif action == "get_info":
                return self.get_audio_info(request)
            elif action == "search":
                return self.search_audio(request)
            else:
                return {"error": f"Unknown action: {action}"}
                
        except Exception as e:
            logger.error(f"Error processing request: {e}")
            return {"error": str(e)}
    
    def download_audio(self, request: Dict[str, Any]) -> Dict[str, Any]:
        """Download audio using yt-dlp (without metadata embedding)"""
        try:
            url = request["url"]
            output_path = Path(request["output_path"])
            format_preference = request.get("format", "mp3")
            quality = request.get("quality", "best")
            title = request.get("title", "")
            artist = request.get("artist", "")
            album = request.get("album")
            year = request.get("year")
            genre = request.get("genre")
            thumbnail_url = request.get("thumbnail_url")
            
            # If URL is empty, search for the track first
            if not url or url.strip() == "":
                logger.info(f"No URL provided, searching for: {artist} - {title}")
                search_query = f"{artist} {title}"
                search_request = {
                    "action": "search",
                    "query": search_query,
                    "limit": 1,
                    "deep_search": False,
                    "platforms": ["youtube"]
                }
                
                search_result = self.search_audio(search_request)
                logger.info(f"Search result: {search_result}")
                if search_result.get("success") and search_result.get("tracks"):
                    tracks = search_result["tracks"]
                    if tracks and len(tracks) > 0:
                        url = tracks[0]["url"]
                        logger.info(f"Found track URL: {url}")
                    else:
                        logger.warning(f"No search results found for: {artist} - {title}")
                        return {"success": False, "error": f"No search results found for: {artist} - {title}"}
                else:
                    error_msg = search_result.get("error", "Search failed")
                    logger.error(f"Search failed: {error_msg}")
                    return {"success": False, "error": f"Search failed: {error_msg}"}
            
            logger.info(f"Starting advanced download: {url}")
            logger.info(f"Track: {artist} - {title}")
            logger.info(f"Quality setting received: '{quality}'")
            
            # Initialize progress tracking
            progress_info = {
                "stage": "initializing",
                "progress": 0.0,
                "message": "Starting download...",
                "downloaded_bytes": 0,
                "total_bytes": None,
                "speed": None,
                "eta": None
            }
            
            # Create proper folder structure relative to download path
            download_dir = Path(request.get("download_dir", "."))
            
            tracks_dir = download_dir / "tracks"
            temp_dir = download_dir / "temp"
            lyrics_dir = download_dir / "lyrics"
            
            # Create progress file for tracking
            task_id = request.get("task_id", str(uuid.uuid4()))
            progress_file = temp_dir / f"progress_{task_id}.json"
            self._write_progress_file(progress_file, progress_info)
            
            tracks_dir.mkdir(exist_ok=True)
            temp_dir.mkdir(exist_ok=True)
            lyrics_dir.mkdir(exist_ok=True)
            
            # Update progress
            progress_info.update({
                "stage": "preparing",
                "progress": 10.0,
                "message": "Preparing download..."
            })
            self._write_progress_file(progress_file, progress_info)
            
            # Generate temporary filename
            temp_filename = f"temp_{uuid.uuid4()}.%(ext)s"
            temp_file = temp_dir / temp_filename
            
            # Convert quality string to yt-dlp format
            ytdlp_quality = self._convert_quality_for_ytdlp(quality)
            
            # Build optimized yt-dlp command with SponsorBlock
            cmd = [
                "yt-dlp",
                "--extract-audio",
                "--audio-format", format_preference,
                "--audio-quality", ytdlp_quality,
                "--output", str(temp_file),
                "--no-playlist",
                "--no-warnings",
                "--ignore-errors",
                "--no-check-certificate",  # Skip cert checks for speed
                "--prefer-free-formats",   # Prefer free formats
                "--extractor-retries", "1",  # Reduce retries
                "--fragment-retries", "1",   # Reduce fragment retries
                "--sponsorblock-remove", "sponsor,intro,outro,selfpromo,preview,interaction,music_offtopic",
                "--user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
                url
            ]
            
            # Add additional options based on format
            if format_preference == "mp3":
                bitrate = self._get_bitrate(quality)
                cmd.extend(["--postprocessor-args", f"ffmpeg:-b:a {bitrate}k"])
            elif format_preference == "m4a":
                bitrate = self._get_bitrate(quality)
                cmd.extend(["--postprocessor-args", f"ffmpeg:-c:a aac -b:a {bitrate}k"])
            
            logger.info(f"Running yt-dlp command: {' '.join(cmd)}")
            
            # Update progress
            progress_info.update({
                "stage": "downloading",
                "progress": 20.0,
                "message": "Downloading audio..."
            })
            self._write_progress_file(progress_file, progress_info)
            
            # Execute yt-dlp for download (not search)
            result = self._run_ytdlp_download(cmd, progress_file, progress_info)
            
            # Find the downloaded file - check for any audio files in temp directory
            downloaded_files = []
            logger.info(f"Looking for downloaded files in: {temp_dir}")
            
            # List all files in temp directory for debugging
            all_files = list(temp_dir.glob("*"))
            logger.info(f"All files in temp directory: {[f.name for f in all_files]}")
            
            # Look for files with audio extensions
            for ext in ['.mp3', '.m4a', '.flac', '.wav', '.ogg', '.webm', '.opus']:
                files = list(temp_dir.glob(f"*{ext}"))
                if files:
                    downloaded_files = files
                    logger.info(f"Found {len(files)} files with extension {ext}: {[f.name for f in files]}")
                    break
            
            if not downloaded_files:
                logger.error(f"No audio files found in {temp_dir}")
                logger.error(f"yt-dlp return code: {result.returncode}")
                logger.error(f"yt-dlp stdout: {result.stdout}")
                logger.error(f"yt-dlp stderr: {result.stderr}")
                return {"success": False, "error": "No audio file found after download"}
            
            downloaded_file = downloaded_files[0]
            
            # Step 1: Move to tracks folder with sanitized filename
            sanitized_filename = self._sanitize_track_filename(artist, title)
            final_output_path = tracks_dir / f"{sanitized_filename}.{downloaded_file.suffix[1:]}"
            shutil.move(str(downloaded_file), str(final_output_path))
            output_path = final_output_path
            
            # Update progress
            progress_info.update({
                "stage": "processing",
                "progress": 60.0,
                "message": "Processing metadata..."
            })
            self._write_progress_file(progress_file, progress_info)
            
            # Download completed - no metadata embedding here
            # Metadata will be embedded by Rust backend after enhanced metadata search
            
            # Clean up info file if it exists
            info_file = downloaded_file.with_suffix('.info.json')
            if info_file.exists():
                info_file.unlink()
            
            # Final progress update
            progress_info.update({
                "stage": "completed",
                "progress": 100.0,
                "message": "Download completed successfully!"
            })
            self._write_progress_file(progress_file, progress_info)
            
            # Clean up progress file
            if progress_file.exists():
                progress_file.unlink()
            
            logger.info(f"Advanced download completed: {output_path}")
            return {
                "success": True,
                "output_path": str(output_path),
                "file_size": output_path.stat().st_size,
                "format": format_preference,
                "quality": quality
            }
            
        except subprocess.CalledProcessError as e:
            logger.error(f"yt-dlp failed: {e.stderr}")
            return {"success": False, "error": f"Download failed: {e.stderr}"}
        except Exception as e:
            logger.error(f"Download error: {e}")
            return {"success": False, "error": str(e)}

    def embed_metadata_only(self, request: Dict[str, Any]) -> Dict[str, Any]:
        """Embed metadata into an existing audio file"""
        try:
            file_path = Path(request["file_path"])
            metadata = request["metadata"]
            
            logger.info(f"Embedding metadata into: {file_path}")
            logger.info(f"File path exists: {file_path.exists()}")
            logger.info(f"File path absolute: {file_path.absolute()}")
            logger.info(f"File path as string: {str(file_path)}")
            logger.info(f"File path bytes: {str(file_path).encode('utf-8')}")
            logger.info(f"Metadata: {metadata}")
            
            # Check if file exists before proceeding
            if not file_path.exists():
                logger.error(f"File does not exist: {file_path}")
                # Try to find similar files in the directory
                parent_dir = file_path.parent
                if parent_dir.exists():
                    logger.info(f"Listing files in directory: {parent_dir}")
                    try:
                        for file in parent_dir.iterdir():
                            if file.is_file() and file.suffix.lower() in ['.mp3', '.m4a', '.flac', '.wav']:
                                logger.info(f"Found audio file: {file}")
                    except Exception as e:
                        logger.error(f"Error listing directory: {e}")
                return {"success": False, "error": f"File does not exist: {file_path}"}
            
            # Embed metadata using mutagen
            embed_result = self.embed_metadata({
                "file_path": str(file_path),
                "metadata": metadata
            })
            
            if not embed_result.get("success"):
                logger.warning(f"Metadata embedding failed: {embed_result.get('error')}")
                return {"success": False, "error": embed_result.get('error', 'Unknown embedding error')}
            
            # Download and embed cover art if available
            if metadata.get("cover_art_url"):
                cover_art = self._download_cover_art(
                    metadata["cover_art_url"], 
                    metadata.get("artist", ""), 
                    metadata.get("title", ""), 
                    metadata.get("album")
                )
                if cover_art:
                    self._embed_cover_art(str(file_path), cover_art)
            
            # Embed lyrics if available (after cover art to avoid overwriting)
            if metadata.get("lyrics"):
                lyrics_result = self.embed_lyrics({
                    "file_path": str(file_path),
                    "lyrics": metadata["lyrics"]
                })
                if not lyrics_result.get("success"):
                    logger.warning(f"Lyrics embedding failed: {lyrics_result.get('error')}")
                else:
                    logger.info("Lyrics embedded successfully")
            
            logger.info(f"Metadata embedding completed: {file_path}")
            return {
                "success": True,
                "file_path": str(file_path)
            }
            
        except Exception as e:
            logger.error(f"Metadata embedding error: {e}")
            return {"success": False, "error": str(e)}
    
    def transcode_audio(self, request: Dict[str, Any]) -> Dict[str, Any]:
        """Transcode audio using FFmpeg"""
        try:
            input_path = Path(request["input_path"])
            output_path = Path(request["output_path"])
            output_format = request.get("format", "mp3")
            quality = request.get("quality", "high")
            
            if not input_path.exists():
                return {"error": "Input file does not exist"}
            
            # Ensure output directory exists
            output_path.parent.mkdir(parents=True, exist_ok=True)
            
            # Check FFmpeg availability
            ffmpeg_path = self._find_ffmpeg_executable()
            if not ffmpeg_path:
                return {"error": "FFmpeg not found"}
            
            # Test FFmpeg functionality
            if not self._test_ffmpeg_functionality(ffmpeg_path):
                return {"error": "FFmpeg is not working properly"}
            
            # Set FFmpeg path for ffmpeg-python
            ffmpeg.set_ffmpeg_path(ffmpeg_path)
            
            # Build FFmpeg stream
            stream = ffmpeg.input(str(input_path))
            
            # Add format-specific options
            if output_format == "mp3":
                bitrate = self._get_bitrate(quality)
                stream = ffmpeg.output(stream, str(output_path), acodec='libmp3lame', audio_bitrate=f'{bitrate}k')
            elif output_format == "m4a":
                bitrate = self._get_bitrate(quality)
                stream = ffmpeg.output(stream, str(output_path), acodec='aac', audio_bitrate=f'{bitrate}k')
            elif output_format == "flac":
                stream = ffmpeg.output(stream, str(output_path), acodec='flac')
            elif output_format == "wav":
                stream = ffmpeg.output(stream, str(output_path), acodec='pcm_s16le')
            
            # Overwrite output file
            stream = ffmpeg.overwrite_output(stream)
            
            logger.info(f"Running FFmpeg transcoding: {input_path} -> {output_path}")
            
            # Execute FFmpeg
            ffmpeg.run(stream, capture_stdout=True, capture_stderr=True)
            
            return {
                "success": True,
                "output_path": str(output_path),
                "file_size": output_path.stat().st_size
            }
            
        except ffmpeg.Error as e:
            logger.error(f"FFmpeg error: {e.stderr.decode() if e.stderr else str(e)}")
            return {"error": f"Transcoding failed: {e.stderr.decode() if e.stderr else str(e)}"}
        except Exception as e:
            logger.error(f"Transcoding error: {e}")
            return {"error": str(e)}
    
    def embed_metadata(self, request: Dict[str, Any]) -> Dict[str, Any]:
        """Embed metadata using mutagen"""
        try:
            if not MUTAGEN_AVAILABLE:
                return {"error": "mutagen not available - cannot embed metadata"}
                
            file_path = Path(request["file_path"])
            metadata = request["metadata"]
            
            if not file_path.exists():
                return {"error": "File does not exist"}
            
            # Import mutagen based on file extension
            file_ext = file_path.suffix.lower()
            
            if file_ext == ".mp3":
                from mutagen.id3 import ID3, TIT2, TPE1, TALB, TYER, TCON, TRCK, TPOS, TCOM, TSRC
                from mutagen.mp3 import MP3
                
                audio = MP3(str(file_path), ID3=ID3)
                
                if metadata.get("title"):
                    audio.tags.add(TIT2(encoding=3, text=metadata["title"]))
                if metadata.get("artist"):
                    audio.tags.add(TPE1(encoding=3, text=metadata["artist"]))
                if metadata.get("album"):
                    audio.tags.add(TALB(encoding=3, text=metadata["album"]))
                if metadata.get("year"):
                    audio.tags.add(TYER(encoding=3, text=str(metadata["year"])))
                if metadata.get("genre"):
                    audio.tags.add(TCON(encoding=3, text=metadata["genre"]))
                if metadata.get("track_number"):
                    audio.tags.add(TRCK(encoding=3, text=str(metadata["track_number"])))
                if metadata.get("disc_number"):
                    audio.tags.add(TPOS(encoding=3, text=str(metadata["disc_number"])))
                if metadata.get("composer"):
                    audio.tags.add(TCOM(encoding=3, text=metadata["composer"]))
                if metadata.get("isrc"):
                    audio.tags.add(TSRC(encoding=3, text=metadata["isrc"]))
                
                audio.save()
                
            elif file_ext in [".m4a", ".mp4"]:
                from mutagen.mp4 import MP4
                from mutagen.mp3 import MP3
                
                logger.info(f"Attempting to load M4A file for metadata: {file_path}")
                try:
                    audio = MP4(str(file_path))
                    logger.info(f"Successfully loaded M4A file: {file_path}")
                except Exception as e:
                    logger.warning(f"Failed to load as M4A file {file_path}: {e}")
                    logger.info(f"Trying to load as MP3 file instead...")
                    try:
                        # Try loading as MP3 if M4A fails
                        audio = MP3(str(file_path))
                        logger.info(f"Successfully loaded as MP3 file: {file_path}")
                        # Use MP3 format for this file
                        file_ext = ".mp3"
                    except Exception as e2:
                        logger.error(f"Failed to load as both M4A and MP3: {e2}")
                        return {"error": f"Failed to load audio file: {e}"}
                
                if file_ext == ".mp3":
                    # Handle as MP3 file
                    if metadata.get("title"):
                        audio.tags.add(TIT2(encoding=3, text=metadata["title"]))
                    if metadata.get("artist"):
                        audio.tags.add(TPE1(encoding=3, text=metadata["artist"]))
                    if metadata.get("album"):
                        audio.tags.add(TALB(encoding=3, text=metadata["album"]))
                    if metadata.get("year"):
                        audio.tags.add(TYER(encoding=3, text=str(metadata["year"])))
                    if metadata.get("genre"):
                        audio.tags.add(TCON(encoding=3, text=metadata["genre"]))
                    if metadata.get("track_number"):
                        audio.tags.add(TRCK(encoding=3, text=str(metadata["track_number"])))
                    if metadata.get("disc_number"):
                        audio.tags.add(TPOS(encoding=3, text=str(metadata["disc_number"])))
                    if metadata.get("album_artist"):
                        audio.tags.add(TPE2(encoding=3, text=metadata["album_artist"]))
                    if metadata.get("composer"):
                        audio.tags.add(TCOM(encoding=3, text=metadata["composer"]))
                    if metadata.get("isrc"):
                        audio.tags.add(TSRC(encoding=3, text=metadata["isrc"]))
                    
                    audio.save()
                else:
                    # Handle as M4A file
                    if metadata.get("title"):
                        audio["\xa9nam"] = [metadata["title"]]
                    if metadata.get("artist"):
                        audio["\xa9ART"] = [metadata["artist"]]
                    if metadata.get("album"):
                        audio["\xa9alb"] = [metadata["album"]]
                    if metadata.get("year"):
                        audio["\xa9day"] = [str(metadata["year"])]
                    if metadata.get("genre"):
                        audio["\xa9gen"] = [metadata["genre"]]
                    if metadata.get("track_number"):
                        audio["trkn"] = [(metadata["track_number"], 0)]
                    if metadata.get("disc_number"):
                        audio["disk"] = [(metadata["disc_number"], 0)]
                    if metadata.get("composer"):
                        audio["\xa9wrt"] = [metadata["composer"]]
                    if metadata.get("isrc"):
                        audio["----:com.apple.iTunes:ISRC"] = [metadata["isrc"].encode()]
                    
                    audio.save()
                
            elif file_ext == ".flac":
                
                audio = FLAC(str(file_path))
                
                # Basic metadata
                if metadata.get("title"):
                    audio["TITLE"] = metadata["title"]
                if metadata.get("artist"):
                    audio["ARTIST"] = metadata["artist"]
                if metadata.get("album"):
                    audio["ALBUM"] = metadata["album"]
                if metadata.get("year"):
                    audio["DATE"] = str(metadata["year"])
                if metadata.get("genre"):
                    audio["GENRE"] = metadata["genre"]
                if metadata.get("track_number"):
                    audio["TRACKNUMBER"] = str(metadata["track_number"])
                if metadata.get("disc_number"):
                    audio["DISCNUMBER"] = str(metadata["disc_number"])
                if metadata.get("composer"):
                    audio["COMPOSER"] = metadata["composer"]
                if metadata.get("isrc"):
                    audio["ISRC"] = metadata["isrc"]
                
                # Enhanced FLAC-specific metadata
                audio["ORGANIZATION"] = "Spotify Downloader"
                audio["ENCODEDBY"] = "Spotify Downloader v1.0"
                audio["ENCODING"] = "FLAC"
                audio["SOURCEMEDIA"] = "Digital Media"
                
                # Add download timestamp
                from datetime import datetime
                audio["DOWNLOAD_DATE"] = datetime.now().isoformat()
                audio["SOURCE"] = "YouTube"
                
                # Add ReplayGain fields (standard for FLAC)
                audio["REPLAYGAIN_TRACK_GAIN"] = "0.0 dB"
                audio["REPLAYGAIN_TRACK_PEAK"] = "1.0"
                audio["REPLAYGAIN_ALBUM_GAIN"] = "0.0 dB"
                audio["REPLAYGAIN_ALBUM_PEAK"] = "1.0"
                
                # Add comment
                audio["COMMENT"] = "Downloaded with Spotify Downloader - High Quality Audio"
                
                audio.save()
                
            elif file_ext == ".wav":
                # WAV files use ID3v2 tags (same as MP3)
                from mutagen.id3 import ID3, TIT2, TPE1, TALB, TYER, TCON, TRCK, TPOS, TCOM, TSRC, TPE2, TENC, TSO2, TSOA, TXXX
                from mutagen.wave import WAVE
                
                audio = WAVE(str(file_path))
                if not audio.tags:
                    audio.add_tags()
                
                # Basic metadata using ID3v2 tags
                if metadata.get("title"):
                    audio.tags.add(TIT2(encoding=3, text=metadata["title"]))
                if metadata.get("artist"):
                    audio.tags.add(TPE1(encoding=3, text=metadata["artist"]))
                if metadata.get("album"):
                    audio.tags.add(TALB(encoding=3, text=metadata["album"]))
                if metadata.get("year"):
                    audio.tags.add(TYER(encoding=3, text=str(metadata["year"])))
                if metadata.get("genre"):
                    audio.tags.add(TCON(encoding=3, text=metadata["genre"]))
                if metadata.get("track_number"):
                    audio.tags.add(TRCK(encoding=3, text=str(metadata["track_number"])))
                if metadata.get("disc_number"):
                    audio.tags.add(TPOS(encoding=3, text=str(metadata["disc_number"])))
                if metadata.get("album_artist"):
                    audio.tags.add(TPE2(encoding=3, text=metadata["album_artist"]))
                if metadata.get("composer"):
                    audio.tags.add(TCOM(encoding=3, text=metadata["composer"]))
                if metadata.get("isrc"):
                    audio.tags.add(TSRC(encoding=3, text=metadata["isrc"]))
                
                # Enhanced WAV-specific metadata
                audio.tags.add(TENC(encoding=3, text="Spotify Downloader v1.0"))
                audio.tags.add(TSO2(encoding=3, text="Spotify Downloader"))  # Organization
                audio.tags.add(TSOA(encoding=3, text="Digital Media"))  # Source media
                
                # Add download timestamp
                from datetime import datetime
                audio.tags.add(TXXX(encoding=3, desc="DOWNLOAD_DATE", text=datetime.now().isoformat()))
                audio.tags.add(TXXX(encoding=3, desc="SOURCE", text="YouTube"))
                
                # Add comment
                audio.tags.add(TXXX(encoding=3, desc="COMMENT", text="Downloaded with Spotify Downloader - High Quality Audio"))
                
                audio.save()
                
            elif file_ext == ".ogg":
                # OGG files can be either Vorbis (lossy) or FLAC (lossless)
                from mutagen.oggvorbis import OggVorbis
                from mutagen.oggflac import OggFLAC
                
                # Try to detect if it's OGG FLAC or OGG Vorbis
                try:
                    audio = OggFLAC(str(file_path))
                    logger.info("Using OggFLAC for lossless OGG file")
                except:
                    audio = OggVorbis(str(file_path))
                    logger.info("Using OggVorbis for lossy OGG file")
                    
            elif file_ext == ".opus":
                # Opus files use Vorbis comments
                from mutagen.opus import Opus
                
                audio = Opus(str(file_path))
                logger.info("Using Opus for Opus file")
                
                # Basic metadata using Vorbis comments
                if metadata.get("title"):
                    audio["TITLE"] = metadata["title"]
                if metadata.get("artist"):
                    audio["ARTIST"] = metadata["artist"]
                if metadata.get("album"):
                    audio["ALBUM"] = metadata["album"]
                if metadata.get("year"):
                    audio["DATE"] = str(metadata["year"])
                if metadata.get("genre"):
                    audio["GENRE"] = metadata["genre"]
                if metadata.get("track_number"):
                    audio["TRACKNUMBER"] = str(metadata["track_number"])
                if metadata.get("disc_number"):
                    audio["DISCNUMBER"] = str(metadata["disc_number"])
                if metadata.get("album_artist"):
                    audio["ALBUMARTIST"] = metadata["album_artist"]
                if metadata.get("composer"):
                    audio["COMPOSER"] = metadata["composer"]
                if metadata.get("isrc"):
                    audio["ISRC"] = metadata["isrc"]
                
                # Enhanced Opus-specific metadata
                audio["ORGANIZATION"] = "Spotify Downloader"
                audio["ENCODEDBY"] = "Spotify Downloader v1.0"
                audio["ENCODING"] = "Opus"
                audio["SOURCEMEDIA"] = "Digital Media"
                
                # Add download timestamp
                from datetime import datetime
                audio["DOWNLOAD_DATE"] = datetime.now().isoformat()
                audio["SOURCE"] = "YouTube"
                
                # Add comment
                audio["COMMENT"] = "Downloaded with Spotify Downloader - High Quality Audio"
                
                audio.save()
                
            elif file_ext == ".ape":
                # APE files use APEv2 tags
                from mutagen.apev2 import APEv2
                
                audio = APEv2(str(file_path))
                logger.info("Using APEv2 for APE file")
                
                # Basic metadata using APEv2 tags
                if metadata.get("title"):
                    audio["Title"] = metadata["title"]
                if metadata.get("artist"):
                    audio["Artist"] = metadata["artist"]
                if metadata.get("album"):
                    audio["Album"] = metadata["album"]
                if metadata.get("year"):
                    audio["Year"] = str(metadata["year"])
                if metadata.get("genre"):
                    audio["Genre"] = metadata["genre"]
                if metadata.get("track_number"):
                    audio["Track"] = str(metadata["track_number"])
                if metadata.get("disc_number"):
                    audio["Disc"] = str(metadata["disc_number"])
                if metadata.get("album_artist"):
                    audio["Album Artist"] = metadata["album_artist"]
                if metadata.get("composer"):
                    audio["Composer"] = metadata["composer"]
                if metadata.get("isrc"):
                    audio["ISRC"] = metadata["isrc"]
                
                # Enhanced APE-specific metadata
                audio["Organization"] = "Spotify Downloader"
                audio["EncodedBy"] = "Spotify Downloader v1.0"
                audio["Codec"] = "APE"
                audio["SourceMedia"] = "Digital Media"
                
                # Add download timestamp
                from datetime import datetime
                audio["DownloadDate"] = datetime.now().isoformat()
                audio["Source"] = "YouTube"
                
                # Add comment
                audio["Comment"] = "Downloaded with Spotify Downloader - High Quality Audio"
                
                audio.save()
            
            return {"success": True}
            
        except ImportError as e:
            logger.error(f"Mutagen import error: {e}")
            return {"success": False, "error": f"Metadata library not available: {e}"}
        except Exception as e:
            logger.error(f"Metadata embedding error: {e}")
            return {"success": False, "error": str(e)}
    
    def get_audio_info(self, request: Dict[str, Any]) -> Dict[str, Any]:
        """Get audio file information"""
        try:
            file_path = Path(request["file_path"])
            
            if not file_path.exists():
                return {"error": "File does not exist"}
            
            # Use FFprobe to get audio information
            cmd = [
                "ffprobe",
                "-v", "quiet",
                "-print_format", "json",
                "-show_format",
                "-show_streams",
                str(file_path)
            ]
            
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                check=True
            )
            
            info = json.loads(result.stdout)
            
            # Extract relevant information
            format_info = info.get("format", {})
            streams = info.get("streams", [])
            
            audio_stream = None
            for stream in streams:
                if stream.get("codec_type") == "audio":
                    audio_stream = stream
                    break
            
            if not audio_stream:
                return {"error": "No audio stream found"}
            
            duration = float(format_info.get("duration", 0))
            bitrate = int(format_info.get("bit_rate", 0))
            sample_rate = int(audio_stream.get("sample_rate", 0))
            channels = int(audio_stream.get("channels", 0))
            codec = audio_stream.get("codec_name", "unknown")
            
            return {
                "success": True,
                "duration": duration,
                "bitrate": bitrate,
                "sample_rate": sample_rate,
                "channels": channels,
                "codec": codec,
                "file_size": file_path.stat().st_size
            }
            
        except subprocess.CalledProcessError as e:
            logger.error(f"FFprobe failed: {e.stderr}")
            return {"error": f"Failed to get audio info: {e.stderr}"}
        except Exception as e:
            logger.error(f"Audio info error: {e}")
            return {"error": str(e)}
    
    def _get_bitrate(self, quality: str) -> int:
        """Get bitrate based on quality setting"""
        quality_map = {
            "low": 128,
            "medium": 192,
            "high": 256,
            "best": 320
        }
        bitrate = quality_map.get(quality, 256)
        logger.info(f"Quality setting: '{quality}' -> Bitrate: {bitrate} kbps")
        return bitrate
    
    def _convert_quality_for_ytdlp(self, quality: str) -> str:
        """Convert our quality strings to yt-dlp format"""
        quality_map = {
            "low": "9",      # 128kbps
            "medium": "5",   # 192kbps  
            "high": "2",     # 256kbps
            "best": "0"      # 320kbps (best available)
        }
        ytdlp_quality = quality_map.get(quality, "2")  # Default to high
        logger.info(f"Converting quality '{quality}' to yt-dlp format: '{ytdlp_quality}'")
        return ytdlp_quality

    def _parse_ytdlp_progress(self, line: str) -> Optional[float]:
        """Parse progress percentage from yt-dlp output line"""
        import re
        
        # yt-dlp progress patterns:
        # [download] 45.2% of 5.67MiB at 1.23MiB/s ETA 00:03
        # [download] 100% of 5.67MiB in 00:04
        # [download] Destination: filename.mp3
        # [download] 100% of 5.67MiB in 00:04
        
        # Look for percentage pattern
        progress_pattern = r'\[download\]\s+(\d+(?:\.\d+)?)%'
        match = re.search(progress_pattern, line)
        
        if match:
            progress = float(match.group(1))
            logger.debug(f"Parsed progress: {progress}% from line: {line}")
            return progress
        
        return None

    def _parse_filename_metadata(self, filename: str) -> Dict[str, str]:
        """Parse artist and title from filename using common patterns"""
        import re
        
        # Remove file extension
        name_without_ext = Path(filename).stem
        
        # Common filename patterns:
        # "Artist - Title"
        # "Artist - Title (feat. Other Artist)"
        # "Artist - Title [Official Video]"
        # "Artist - Title (Official Music Video)"
        # "Artist - Title - Album"
        # "Artist - Title (Remix)"
        
        patterns = [
            # Standard "Artist - Title" format
            r'^(.+?)\s*-\s*(.+?)(?:\s*\([^)]*\))?(?:\s*\[[^\]]*\])?$',
            # "Artist: Title" format
            r'^(.+?)\s*:\s*(.+?)(?:\s*\([^)]*\))?(?:\s*\[[^\]]*\])?$',
            # "Artist _ Title" format
            r'^(.+?)\s*_\s*(.+?)(?:\s*\([^)]*\))?(?:\s*\[[^\]]*\])?$',
            # "Artist | Title" format
            r'^(.+?)\s*\|\s*(.+?)(?:\s*\([^)]*\))?(?:\s*\[[^\]]*\])?$',
            # "Artist ft. Other - Title" format
            r'^(.+?)\s+ft\.?\s+[^-]+?\s*-\s*(.+?)(?:\s*\([^)]*\))?(?:\s*\[[^\]]*\])?$',
            # "Artist feat. Other - Title" format
            r'^(.+?)\s+feat\.?\s+[^-]+?\s*-\s*(.+?)(?:\s*\([^)]*\))?(?:\s*\[[^\]]*\])?$',
            # Just title (no separator)
            r'^(.+)$'
        ]
        
        for pattern in patterns:
            match = re.match(pattern, name_without_ext, re.IGNORECASE)
            if match:
                if len(match.groups()) == 2:
                    artist = match.group(1).strip()
                    title = match.group(2).strip()
                    
                    # Clean up common suffixes/prefixes
                    title = re.sub(r'\s*\([^)]*\)\s*$', '', title)  # Remove trailing (text)
                    title = re.sub(r'\s*\[[^\]]*\]\s*$', '', title)  # Remove trailing [text]
                    title = re.sub(r'\s*-\s*[^-]*$', '', title)  # Remove trailing - text
                    
                    # Clean up artist
                    artist = re.sub(r'\s*\([^)]*\)\s*$', '', artist)  # Remove trailing (text)
                    artist = re.sub(r'\s*\[[^\]]*\]\s*$', '', artist)  # Remove trailing [text]
                    
                    logger.info(f"Parsed filename '{filename}': Artist='{artist}', Title='{title}'")
                    return {"artist": artist, "title": title}
                elif len(match.groups()) == 1:
                    # Only title found
                    title = match.group(1).strip()
                    title = re.sub(r'\s*\([^)]*\)\s*$', '', title)
                    title = re.sub(r'\s*\[[^\]]*\]\s*$', '', title)
                    logger.info(f"Parsed filename '{filename}': Title='{title}' (no artist)")
                    return {"artist": "", "title": title}
        
        # Fallback: use entire filename as title
        logger.warning(f"Could not parse filename '{filename}', using as title")
        return {"artist": "", "title": name_without_ext}

    def _test_filename_parsing(self):
        """Test function for filename parsing (for debugging)"""
        test_cases = [
            "Artist - Title.mp3",
            "Artist - Title (feat. Other).mp3", 
            "Artist - Title [Official Video].mp3",
            "Artist: Title.mp3",
            "Artist _ Title.mp3",
            "Artist | Title.mp3",
            "Artist ft. Other - Title.mp3",
            "Artist feat. Other - Title.mp3",
            "Just Title.mp3",
            "Artist - Title (Remix) [Official].mp3"
        ]
        
        logger.info("Testing filename parsing:")
        for test_case in test_cases:
            result = self._parse_filename_metadata(test_case)
            logger.info(f"  '{test_case}' -> {result}")
    
    def search_audio(self, request: Dict[str, Any]) -> Dict[str, Any]:
        """Search for audio using yt-dlp across all platforms"""
        try:
            query = request["query"]
            limit = request.get("limit", 10)
            deep_search = request.get("deep_search", False)
            
            logger.info(f"Starting search: query='{query}', limit={limit}, deep_search={deep_search}")
            
            # Test yt-dlp availability first
            try:
                test_result = subprocess.run(["yt-dlp", "--version"], capture_output=True, text=True, timeout=10)
                if test_result.returncode != 0:
                    logger.error(f"yt-dlp not working: {test_result.stderr}")
                    return {"success": False, "error": "yt-dlp is not working properly"}
                logger.info(f"yt-dlp version: {test_result.stdout.strip()}")
            except Exception as e:
                logger.error(f"yt-dlp test failed: {e}")
                return {"success": False, "error": f"yt-dlp test failed: {e}"}
            
            if deep_search:
                result = self._deep_search(query, limit)
            else:
                result = self._single_search(query, limit)
            
            logger.info(f"Search completed: found {result.get('total', 0)} tracks")
            return result
                
        except Exception as e:
            logger.error(f"Error searching audio: {e}")
            return {"success": False, "error": str(e)}
    
    def _single_search(self, query: str, limit: int) -> Dict[str, Any]:
        """Single search across all platforms using yt-dlp"""
        try:
            all_tracks = []
            proxy = None  # Initialize proxy variable
            
            # Search YouTube - Single optimized search
            try:
                cmd = [
                    "yt-dlp",
                    "--dump-json",
                    "--flat-playlist",
                    "--no-download",
                    "--max-downloads", "3",  # Fixed to 3 results
                    f"ytsearch3:{query}",
                ]
                
                # Add optimized options for faster search
                cmd.extend([
                    "--default-search", "ytsearch",
                    "--ignore-errors",
                    "--no-warnings",
                    "--user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
                    "--no-playlist",  # Skip playlists for faster search
                    "--no-check-certificate",  # Skip cert checks for speed
                    "--prefer-free-formats",  # Prefer free formats
                    "--extractor-retries", "1",  # Reduce retries
                    "--fragment-retries", "1",  # Reduce fragment retries
                ])
                
                # Add proxy if available
                if proxy:
                    cmd.extend(["--proxy", proxy])
                logger.info(f"Searching YouTube (regular): {query}")
                logger.info(f"Full command: {' '.join(cmd)}")
                result = self._run_ytdlp(cmd)
                logger.info(f"yt-dlp result: returncode={result.returncode}, stdout_length={len(result.stdout)}")
                if result.returncode == 0:
                    tracks = self._parse_search_results(result.stdout)
                    all_tracks.extend(tracks)
                elif result.returncode == 124:
                    logger.warning("YouTube regular search timed out")
                else:
                    logger.warning(f"YouTube regular search failed with code {result.returncode}")
            except Exception as e:
                logger.warning(f"YouTube search failed: {e}")
            
            # Search SoundCloud
            try:
                cmd = [
                    "yt-dlp",
                    "--dump-json",
                    "--flat-playlist",
                    "--no-download",
                    "--max-downloads", str(limit),
                    f"scsearch{limit}:{query}",
                    "--user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
                ]
                
                # Add proxy if available
                if proxy:
                    cmd.extend(["--proxy", proxy])
                logger.info(f"Searching SoundCloud: {query}")
                result = self._run_ytdlp(cmd)
                if result.returncode == 0:
                    tracks = self._parse_search_results(result.stdout)
                    all_tracks.extend(tracks)
                elif result.returncode == 124:
                    logger.warning("SoundCloud search timed out")
                else:
                    logger.warning(f"SoundCloud search failed with code {result.returncode}")
            except Exception as e:
                logger.warning(f"SoundCloud search failed: {e}")
            
            # If no tracks found, try a simple fallback search
            if not all_tracks:
                logger.warning("No YouTube results, trying SoundCloud fallback...")
                try:
                    # SoundCloud fallback search
                    fallback_cmd = [
                        "yt-dlp",
                        "--dump-json",
                        "--flat-playlist",
                        "--no-download",
                        "--max-downloads", "3",
                        f"scsearch3:{query}",
                        "--user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
                    ]
                    
                    if proxy:
                        fallback_cmd.extend(["--proxy", proxy])
                    
                    logger.info(f"Fallback search command: {' '.join(fallback_cmd)}")
                    result = self._run_ytdlp(fallback_cmd)
                    logger.info(f"Fallback result: returncode={result.returncode}, stdout_length={len(result.stdout)}")
                    
                    if result.returncode == 0:
                        tracks = self._parse_search_results(result.stdout)
                        all_tracks.extend(tracks)
                        logger.info(f"Fallback search found {len(tracks)} tracks")
                except Exception as e:
                    logger.warning(f"Fallback search failed: {e}")
            
            # Apply fuzzy matching and ranking
            ranked_tracks = self._rank_search_results(all_tracks, query)
            
            return {
                "success": True,
                "tracks": ranked_tracks,
                "total": len(ranked_tracks),
                "query": query,
                "search_type": "single"
            }
            
        except Exception as e:
            logger.error(f"Error in single search: {e}")
            return {"success": False, "error": str(e)}
    
    def _deep_search(self, query: str, limit: int) -> Dict[str, Any]:
        """Deep search with multiple search strategies and platforms"""
        try:
            all_tracks = []
            proxy = None  # Initialize proxy variable
            # Optimized 3-step search: ytsearch3 → scsearch3 → ytsearch3 with "music"
            search_strategies = [
                # Primary YouTube search (most reliable)
                ("youtube", f"ytsearch3:{query}"),
                
                # Fallback SoundCloud search
                ("soundcloud", f"scsearch3:{query}"),
                
                # Alternative YouTube search with "music" keyword
                ("youtube_alt", f"ytsearch3:\"{query} music\""),
            ]
            
            for platform, search_url in search_strategies:
                try:
                    cmd = [
                        "yt-dlp",
                        "--dump-json",
                        "--flat-playlist",
                        "--no-download",
                        "--max-downloads", str(limit),
                        search_url,
                        "--user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
                    ]
                    
                    logger.info(f"Deep search - {platform}: {search_url}")
                    result = self._run_ytdlp(cmd)
                    
                    if result.returncode == 0:
                        tracks = self._parse_search_results(result.stdout)
                        # Mark lyrics versions for priority ranking
                        if platform == "youtube_lyrics":
                            for track in tracks:
                                track["is_lyrics_version"] = True
                        all_tracks.extend(tracks)
                        
                        # Early exit if we have enough good results
                        if len(all_tracks) >= limit:
                            logger.info(f"Found enough results ({len(all_tracks)}), stopping search early")
                            break
                        
                except Exception as e:
                    logger.warning(f"Deep search failed for {platform}: {e}")
                    continue
            
            # Remove duplicates based on URL and title
            unique_tracks = self._deduplicate_tracks(all_tracks)
            
            # Apply fuzzy matching and ranking
            ranked_tracks = self._rank_search_results(unique_tracks, query)
            
            return {
                "success": True,
                "tracks": ranked_tracks[:limit],  # Limit final results
                "total": len(ranked_tracks),
                "query": query,
                "search_type": "deep"
            }
            
        except Exception as e:
            logger.error(f"Error in deep search: {e}")
            return {"success": False, "error": str(e)}
    
    def _parse_search_results(self, stdout: str) -> List[Dict[str, Any]]:
        """Parse yt-dlp search results from stdout"""
        tracks = []
        logger.info(f"Parsing search results from {len(stdout)} characters of output")
        logger.debug(f"Raw stdout: {stdout[:500]}...")  # Log first 500 chars for debugging
        
        if not stdout.strip():
            logger.warning("No output from yt-dlp search")
            return tracks
            
        for line in stdout.strip().split('\n'):
            if line.strip():
                try:
                    track_data = json.loads(line)
                    logger.debug(f"Parsed track data: {track_data.get('title', 'Unknown')} by {track_data.get('uploader', 'Unknown')}")
                    track = self._parse_track_info(track_data)
                    if track:
                        logger.info(f"Added track: {track['title']} by {track['artist']}")
                        tracks.append(track)
                    else:
                        logger.debug(f"Filtered out track: {track_data.get('title', 'Unknown')}")
                except json.JSONDecodeError as e:
                    logger.debug(f"JSON decode error for line: {line[:100]}... Error: {e}")
                    continue
        logger.info(f"Total tracks parsed: {len(tracks)}")
        return tracks
    
    def _parse_track_info(self, data: Dict[str, Any]) -> Optional[Dict[str, Any]]:
        """Parse yt-dlp output into track info"""
        try:
            # Extract basic info
            title = data.get("title", "Unknown")
            uploader = data.get("uploader", "Unknown")
            duration = data.get("duration", 0)
            thumbnail = data.get("thumbnail", "")
            url = data.get("webpage_url", data.get("url", ""))
            upload_date = data.get("upload_date", "")
            
            # Filter by duration: 1 minute (60s) to 16:30 minutes (990s)
            if duration and (duration < 60 or duration > 990):
                logger.debug(f"Filtering out track '{title}' - duration: {duration}s")
                return None
            
            # Filter out snippets and speed modifications
            title_lower = title.lower()
            snippet_keywords = ['slowed', 'reverb', 'sped up', 'speed up', 'trailer', 'snippet', 'preview', 'teaser', 'short', 'clip']
            if any(keyword in title_lower for keyword in snippet_keywords):
                logger.debug(f"Filtering out track '{title}' - contains snippet keywords")
                return None
            
            # Determine platform
            extractor = data.get("extractor", "").lower()
            if "youtube" in extractor:
                platform = "youtube"
            elif "soundcloud" in extractor:
                platform = "soundcloud"
            elif "bandcamp" in extractor:
                platform = "bandcamp"
            elif "vimeo" in extractor:
                platform = "vimeo"
            else:
                platform = extractor
            
            # Extract year from upload date
            year = None
            if upload_date and len(upload_date) >= 4:
                try:
                    year = int(upload_date[:4])
                except ValueError:
                    pass
            
            return {
                "id": data.get("id", ""),
                "title": title,
                "artist": uploader,
                "album": None,
                "duration": int(duration) if duration else None,
                "thumbnail_url": thumbnail if thumbnail else None,
                "source": platform,
                "url": url,
                "quality": None,
                "format": None,
                "year": year,
                "isrc": None,
                "album_artist": None,
                "track_number": None,
                "disc_number": None,
                "composer": None,
            }
            
        except Exception as e:
            logger.error(f"Error parsing track info: {e}")
            return None
    
    def _deduplicate_tracks(self, tracks: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """Remove duplicate tracks based on URL and title similarity"""
        seen_urls = set()
        seen_titles = set()
        unique_tracks = []
        
        for track in tracks:
            url = track.get("url", "")
            title = track.get("title", "").lower().strip()
            
            # Check for exact URL match
            if url in seen_urls:
                continue
                
            # Check for similar title (basic deduplication)
            if title in seen_titles:
                continue
            
            seen_urls.add(url)
            seen_titles.add(title)
            unique_tracks.append(track)
        
        return unique_tracks
    
    def _get_cached_ffmpeg_path(self) -> Optional[str]:
        """Get cached FFmpeg path or find and cache it"""
        if self._ffmpeg_path is None:
            self._ffmpeg_path = self._find_ffmpeg_executable()
        return self._ffmpeg_path

    def _find_ffmpeg_executable(self) -> Optional[str]:
        """Find FFmpeg executable with comprehensive detection"""
        # Method 1: Try ffmpeg-python detection
        try:
            ffmpeg_path = ffmpeg.get_ffmpeg_path()
            if ffmpeg_path and os.path.exists(ffmpeg_path):
                return ffmpeg_path
        except Exception:
            pass
        
        # Method 2: Environment variable
        ffmpeg_path = os.environ.get('FFMPEG_BINARY')
        if ffmpeg_path and os.path.exists(ffmpeg_path):
            return ffmpeg_path
        
        # Method 3: Hardcoded paths (your specific installation)
        hardcoded_paths = [
            "C:\\Users\\temaz\\Downloads\\ffmpeg-master-latest-win64-gpl-shared\\bin\\ffmpeg.exe",
            "C:\\ffmpeg\\bin\\ffmpeg.exe",
            "C:\\Program Files\\ffmpeg\\bin\\ffmpeg.exe",
            "C:\\Program Files (x86)\\ffmpeg\\bin\\ffmpeg.exe"
        ]
        
        for path in hardcoded_paths:
            if os.path.exists(path):
                return path
        
        # Method 4: PATH environment variable
        try:
            result = subprocess.run(['where', 'ffmpeg'], capture_output=True, text=True, timeout=10)
            if result.returncode == 0:
                path = result.stdout.strip().split('\n')[0]
                if os.path.exists(path):
                    return path
        except Exception as e:
            logger.debug(f"PATH detection failed: {e}")
        
        # Method 5: Try common names
        common_names = ['ffmpeg.exe', 'ffmpeg', 'ffmpeg.bat']
        for name in common_names:
            try:
                result = subprocess.run([name, '-version'], capture_output=True, text=True, timeout=5)
                if result.returncode == 0:
                    # Find the actual executable path
                    where_result = subprocess.run(['where', name], capture_output=True, text=True, timeout=5)
                    if where_result.returncode == 0:
                        path = where_result.stdout.strip().split('\n')[0]
                        return path
            except Exception as e:
                logger.debug(f"Common name '{name}' detection failed: {e}")
        
        return None

    def _test_ffmpeg_functionality(self, ffmpeg_path: str) -> bool:
        """Test if FFmpeg actually works"""
        try:
            # Test ffmpeg
            result = subprocess.run([ffmpeg_path, '-version'], capture_output=True, text=True, timeout=10)
            if result.returncode != 0:
                return False
            
            # Test ffprobe (should be in same directory)
            ffprobe_path = ffmpeg_path.replace('ffmpeg.exe', 'ffprobe.exe')
            if os.path.exists(ffprobe_path):
                probe_result = subprocess.run([ffprobe_path, '-version'], capture_output=True, text=True, timeout=10)
                if probe_result.returncode != 0:
                    return False
            else:
                return False
            
            return True
            
        except Exception:
            return False

    def _run_ytdlp_download(self, cmd: List[str], progress_file: Path = None, progress_info: Dict = None) -> subprocess.CompletedProcess:
        """Run yt-dlp command for actual downloads (not search) with longer timeout"""
        try:
            logger.info("=" * 60)
            logger.info("STARTING YT-DLP DOWNLOAD")
            logger.info("=" * 60)
            
            # Get cached FFmpeg path (skip functionality test for speed)
            ffmpeg_path = self._get_cached_ffmpeg_path()
            if not ffmpeg_path:
                raise Exception("FFmpeg not found - cannot download audio")
            
            # Prepare environment
            env = os.environ.copy()
            env['FFMPEG_BINARY'] = ffmpeg_path
            
            # Get FFmpeg directory for yt-dlp
            ffmpeg_dir = os.path.dirname(ffmpeg_path)
            
            # Build the complete yt-dlp command
            ytdlp_cmd = cmd + ['--ffmpeg-location', ffmpeg_dir]
            
            
            # Execute yt-dlp with optimized timeout for downloads
            logger.info("Executing yt-dlp download...")
            result = subprocess.run(
                ytdlp_cmd,
                capture_output=True,
                text=True,
                env=env,
                timeout=180  # 3 minute timeout for downloads (reduced from 5)
            )
            
            logger.info(f"yt-dlp download completed with return code: {result.returncode}")
            logger.info(f"yt-dlp stdout length: {len(result.stdout)}")
            logger.info(f"yt-dlp stderr length: {len(result.stderr)}")
            
            if result.returncode != 0:
                logger.error(f"yt-dlp download failed: {result.stderr}")
            
            return result
            
        except subprocess.TimeoutExpired as e:
            logger.error(f"yt-dlp download timed out: {e}")
            return subprocess.CompletedProcess(cmd, 124, "", "Download timed out")
        except Exception as e:
            logger.error(f"Error running yt-dlp download: {e}")
            return subprocess.CompletedProcess(cmd, 1, "", str(e))

    def _run_ytdlp(self, cmd: List[str], progress_file: Path = None, progress_info: Dict = None) -> subprocess.CompletedProcess:
        """Run yt-dlp command with robust FFmpeg detection and error handling"""
        try:
            logger.info("=" * 60)
            logger.info("STARTING YT-DLP EXECUTION")
            logger.info("=" * 60)
            
            # Find FFmpeg
            ffmpeg_path = self._find_ffmpeg_executable()
            if not ffmpeg_path:
                raise Exception("FFmpeg not found - cannot download audio")
            
            # Test FFmpeg functionality
            if not self._test_ffmpeg_functionality(ffmpeg_path):
                raise Exception("FFmpeg is not working properly")
            
            # Prepare environment
            env = os.environ.copy()
            env['FFMPEG_BINARY'] = ffmpeg_path
            
            # Get FFmpeg directory for yt-dlp
            ffmpeg_dir = os.path.dirname(ffmpeg_path)
            
            # Build the complete yt-dlp command
            ytdlp_cmd = cmd + ['--ffmpeg-location', ffmpeg_dir]
            
            
            # Execute yt-dlp with real-time progress capture and timeout
            logger.info("Executing yt-dlp...")
            process = subprocess.Popen(
                ytdlp_cmd,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                text=True,
                env=env,
                bufsize=1,
                universal_newlines=True
            )
            
            # Capture output in real-time and parse progress with timeout
            stdout_lines = []
            import time
            start_time = time.time()
            timeout_seconds = 5  # 5 second timeout for search operations
            
            while True:
                # Check for timeout
                if time.time() - start_time > timeout_seconds:
                    logger.warning(f"yt-dlp search timed out after {timeout_seconds} seconds")
                    process.terminate()
                    try:
                        process.wait(timeout=5)  # Give it 5 seconds to terminate gracefully
                    except subprocess.TimeoutExpired:
                        process.kill()  # Force kill if it doesn't terminate
                    return subprocess.CompletedProcess(ytdlp_cmd, 124, "", "Timeout")  # Return timeout code
                
                output = process.stdout.readline()
                if output == '' and process.poll() is not None:
                    break
                if output:
                    line = output.strip()
                    stdout_lines.append(line)
                    logger.info(f"yt-dlp: {line}")
                    
                    # Parse progress from yt-dlp output
                    if progress_file and progress_info:
                        progress = self._parse_ytdlp_progress(line)
                        if progress is not None:
                            progress_info.update({
                                "stage": "downloading",
                                "progress": progress,
                                "message": f"Downloading... {progress:.1f}%"
                            })
                            self._write_progress_file(progress_file, progress_info)
            
            # Wait for process to complete
            return_code = process.wait()
            
            logger.info(f"yt-dlp exit code: {return_code}")
            
            if return_code != 0:
                error_msg = f"yt-dlp failed with exit code {return_code}"
                if stdout_lines:
                    error_msg += f": {' '.join(stdout_lines[-5:])}"  # Last 5 lines for context
                logger.error(error_msg)
                raise Exception(error_msg)
            
            logger.info("✓ yt-dlp executed successfully")
            # Create a mock result object for compatibility
            class MockResult:
                def __init__(self, returncode, stdout_lines):
                    self.returncode = returncode
                    self.stdout = '\n'.join(stdout_lines)
                    self.stderr = ""
            
            return MockResult(return_code, stdout_lines)
            
        except subprocess.TimeoutExpired:
            logger.error("yt-dlp command timed out after 120 seconds")
            raise Exception("yt-dlp command timed out")
        except Exception as e:
            logger.error(f"Error running yt-dlp: {e}")
            raise

    def embed_cover_art(self, request: Dict[str, Any]) -> Dict[str, Any]:
        """Embed cover art using mutagen"""
        try:
            if not MUTAGEN_AVAILABLE:
                return {"error": "mutagen not available - cannot embed cover art"}
                
            file_path = Path(request["file_path"])
            cover_art = request["cover_art"]
            
            if not file_path.exists():
                return {"error": "File does not exist"}
            
            # Use pre-downloaded data if available, otherwise download from URL
            if cover_art.get("data"):
                logger.info(f"Using pre-downloaded cover art data: {len(cover_art['data'])} bytes")
            elif cover_art.get("url"):
                import requests
                try:
                    logger.info(f"Downloading cover art from: {cover_art['url']}")
                    response = requests.get(cover_art["url"], timeout=10)
                    response.raise_for_status()  # Raise an exception for bad status codes
                    cover_art["data"] = response.content
                    cover_art["mime_type"] = response.headers.get("content-type", "image/jpeg")
                    logger.info(f"Successfully downloaded cover art: {len(cover_art['data'])} bytes, type: {cover_art['mime_type']}")
                except requests.exceptions.RequestException as e:
                    logger.error(f"Failed to download cover art from {cover_art['url']}: {e}")
                    return {"error": f"Failed to download cover art: {e}"}
                except Exception as e:
                    logger.error(f"Unexpected error downloading cover art: {e}")
                    return {"error": f"Unexpected error downloading cover art: {e}"}
            
            if not cover_art.get("data"):
                return {"error": "No cover art data available"}
            
            # Import mutagen based on file extension
            file_ext = file_path.suffix.lower()
            
            if not MUTAGEN_AVAILABLE:
                return {"error": "mutagen not available"}
                
            if file_ext == ".mp3":
                
                audio = MP3(str(file_path), ID3=ID3)
                audio.tags.add(APIC(
                    encoding=3,
                    mime=cover_art.get("mime_type", "image/jpeg"),
                    type=3,  # Cover (front)
                    desc="Cover",
                    data=cover_art["data"]
                ))
                audio.save()
                
            elif file_ext in [".m4a", ".mp4"]:
                from mutagen.mp4 import MP4
                
                audio = MP4(str(file_path))
                audio["covr"] = [cover_art["data"]]
                audio.save()
                
            elif file_ext == ".flac":
                
                audio = FLAC(str(file_path))
                picture = mutagen.flac.Picture()
                picture.type = 3  # Cover (front)
                picture.mime = cover_art.get("mime_type", "image/jpeg")
                picture.data = cover_art["data"]
                
                # Add picture metadata for better compatibility
                picture.width = 600
                picture.height = 600
                picture.depth = 24
                picture.colors = 0  # Unknown
                picture.description = "Front Cover"
                
                # Clear existing pictures to avoid duplicates
                audio.clear_pictures()
                audio.add_picture(picture)
                audio.save()
                
            elif file_ext == ".wav":
                # WAV files don't support embedded cover art
                # Instead, we'll save the cover art as an external image file
                from mutagen.id3 import APIC
                from mutagen.wave import WAVE
                
                audio = WAVE(str(file_path))
                if not audio.tags:
                    audio.add_tags()
                
                # Try to embed as APIC (some players might support it)
                try:
                    audio.tags.add(APIC(
                        encoding=3,
                        mime=cover_art.get("mime_type", "image/jpeg"),
                        type=3,  # Cover (front)
                        desc="Cover",
                        data=cover_art["data"]
                    ))
                    audio.save()
                    logger.info("Cover art embedded in WAV file (APIC frame)")
                except Exception as e:
                    logger.warning(f"Could not embed cover art in WAV file: {e}")
                    # Save as external image file
                    self._save_external_cover_art(file_path, cover_art)
                    
            elif file_ext == ".ogg":
                # OGG files support cover art via Vorbis comments (both Vorbis and FLAC)
                from mutagen.oggvorbis import OggVorbis
                from mutagen.oggflac import OggFLAC
                
                # Try to detect if it's OGG FLAC or OGG Vorbis
                try:
                    audio = OggFLAC(str(file_path))
                    logger.info("Using OggFLAC for lossless OGG cover art")
                except:
                    audio = OggVorbis(str(file_path))
                    logger.info("Using OggVorbis for lossy OGG cover art")
                
                # Embed cover art using Vorbis comment with base64 encoding
                import base64
                cover_data_b64 = base64.b64encode(cover_art["data"]).decode('utf-8')
                mime_type = cover_art.get("mime_type", "image/jpeg")
                
                # Add cover art as Vorbis comment
                audio["METADATA_BLOCK_PICTURE"] = [cover_data_b64]
                audio["COVERART"] = [cover_data_b64]  # Alternative field
                audio["COVERARTMIME"] = [mime_type]
                
                audio.save()
                logger.info("Cover art embedded in OGG file (Vorbis comment)")
                
            elif file_ext == ".opus":
                # Opus files support cover art via Vorbis comments
                from mutagen.opus import Opus
                
                audio = Opus(str(file_path))
                
                # Embed cover art using Vorbis comment with base64 encoding
                import base64
                cover_data_b64 = base64.b64encode(cover_art["data"]).decode('utf-8')
                mime_type = cover_art.get("mime_type", "image/jpeg")
                
                # Add cover art as Vorbis comment
                audio["METADATA_BLOCK_PICTURE"] = [cover_data_b64]
                audio["COVERART"] = [cover_data_b64]  # Alternative field
                audio["COVERARTMIME"] = [mime_type]
                
                audio.save()
                logger.info("Cover art embedded in Opus file (Vorbis comment)")
                
            elif file_ext == ".ape":
                # APE files support cover art via APEv2 tags
                from mutagen.apev2 import APEv2
                
                audio = APEv2(str(file_path))
                
                # Embed cover art using APEv2 with base64 encoding
                import base64
                cover_data_b64 = base64.b64encode(cover_art["data"]).decode('utf-8')
                mime_type = cover_art.get("mime_type", "image/jpeg")
                
                # Add cover art as APEv2 tag
                audio["Cover Art (Front)"] = [cover_data_b64]
                audio["Cover Art MIME Type"] = [mime_type]
                
                audio.save()
                logger.info("Cover art embedded in APE file (APEv2)")
                
            return {"success": True}
            
        except Exception as e:
            logger.error(f"Error embedding cover art: {e}")
            return {"error": str(e)}

    def _format_lyrics(self, lyrics: str) -> str:
        """Format lyrics with proper line breaks"""
        if not lyrics:
            return ""
        
        # First, try to split by newlines
        lines = lyrics.strip().split('\n')
        
        # If we only have one line, try to split by timestamp patterns
        if len(lines) == 1 and '[' in lyrics and ']' in lyrics:
            # Split by timestamp pattern [MM:SS.XX]
            import re
            # Find all timestamp patterns and split the text
            parts = re.split(r'(\[\d{2}:\d{2}\.\d{2}\])', lyrics)
            formatted_lines = []
            
            # Process parts in pairs: timestamp + text
            for i in range(0, len(parts), 2):
                if i < len(parts) - 1:
                    timestamp = parts[i]
                    text = parts[i + 1] if i + 1 < len(parts) else ""
                    if timestamp and text:
                        # Clean up the text and combine with timestamp
                        clean_text = text.strip()
                        if clean_text:
                            formatted_lines.append(timestamp + " " + clean_text)
                elif i < len(parts) and parts[i].strip():
                    # Handle any remaining text without timestamp
                    formatted_lines.append(parts[i].strip())
        else:
            # Process multiple lines normally
            formatted_lines = []
            for line in lines:
                line = line.strip()
                if line:
                    formatted_lines.append(line)
        
        return '\n'.join(formatted_lines)

    def _create_lrc_file(self, audio_file_path: Path, lyrics: str) -> None:
        """Create LRC file for FLAC/WAV files"""
        try:
            # Get the lyrics directory
            lyrics_dir = audio_file_path.parent.parent / "lyrics"
            lyrics_dir.mkdir(exist_ok=True)
            
            # Create LRC filename based on audio file
            lrc_filename = audio_file_path.stem + ".lrc"
            lrc_path = lyrics_dir / lrc_filename
            
            # Write lyrics to LRC file
            with open(lrc_path, 'w', encoding='utf-8') as f:
                f.write(lyrics)
            
            logger.info(f"Created LRC file: {lrc_path}")
            
        except Exception as e:
            logger.error(f"Error creating LRC file: {e}")

    def _save_external_cover_art(self, audio_file_path: Path, cover_art: Dict[str, Any]) -> None:
        """Save cover art as external image file for WAV files"""
        try:
            # Get the covers directory
            covers_dir = audio_file_path.parent.parent / "covers"
            covers_dir.mkdir(exist_ok=True)
            
            # Determine file extension from MIME type
            mime_type = cover_art.get("mime_type", "image/jpeg")
            if "png" in mime_type:
                ext = ".png"
            elif "gif" in mime_type:
                ext = ".gif"
            else:
                ext = ".jpg"  # Default to JPEG
            
            # Create cover art filename based on audio file
            cover_filename = audio_file_path.stem + ext
            cover_path = covers_dir / cover_filename
            
            # Write cover art data to file
            with open(cover_path, 'wb') as f:
                f.write(cover_art["data"])
            
            logger.info(f"Saved external cover art: {cover_path}")
            
        except Exception as e:
            logger.error(f"Error saving external cover art: {e}")

    def validate_flac_metadata(self, file_path: Path) -> Dict[str, Any]:
        """Validate FLAC metadata after embedding"""
        try:
            if not MUTAGEN_AVAILABLE:
                return {"error": "mutagen not available - cannot validate metadata"}
                
            if not file_path.exists():
                return {"error": "File does not exist"}
            
            file_ext = file_path.suffix.lower()
            if file_ext != ".flac":
                return {"error": "File is not a FLAC file"}
            
            audio = FLAC(str(file_path))
            
            # Check required fields
            required_fields = ["TITLE", "ARTIST", "ALBUM", "TRACKNUMBER"]
            missing_fields = []
            
            for field in required_fields:
                if not audio.get(field):
                    missing_fields.append(field)
            
            # Check cover art
            cover_art_present = len(audio.pictures) > 0
            
            # Check lyrics
            lyrics_present = bool(audio.get("LYRICS") or audio.get("UNSYNCEDLYRICS"))
            
            # Check enhanced metadata
            enhanced_fields = ["ORGANIZATION", "ENCODEDBY", "ENCODING", "REPLAYGAIN_TRACK_GAIN"]
            enhanced_present = all(audio.get(field) for field in enhanced_fields)
            
            validation_result = {
                "success": len(missing_fields) == 0,
                "missing_required_fields": missing_fields,
                "cover_art_present": cover_art_present,
                "lyrics_present": lyrics_present,
                "enhanced_metadata_present": enhanced_present,
                "total_pictures": len(audio.pictures),
                "vorbis_comments_count": len(audio),
                "file_size_mb": round(file_path.stat().st_size / (1024 * 1024), 2)
            }
            
            if missing_fields:
                logger.warning(f"Missing FLAC metadata fields: {missing_fields}")
            
            if not cover_art_present:
                logger.warning("No cover art found in FLAC file")
            
            if not lyrics_present:
                logger.info("No lyrics found in FLAC file")
            
            if not enhanced_present:
                logger.info("Enhanced metadata not fully present")
            
            return validation_result
            
        except Exception as e:
            logger.error(f"Error validating FLAC metadata: {e}")
            return {"error": str(e)}

    def validate_wav_metadata(self, file_path: Path) -> Dict[str, Any]:
        """Validate WAV metadata after embedding"""
        try:
            if not MUTAGEN_AVAILABLE:
                return {"error": "mutagen not available - cannot validate metadata"}
                
            if not file_path.exists():
                return {"error": "File does not exist"}
            
            file_ext = file_path.suffix.lower()
            if file_ext != ".wav":
                return {"error": "File is not a WAV file"}
            
            from mutagen.wave import WAVE
            
            audio = WAVE(str(file_path))
            
            # Check required fields
            required_fields = ["TIT2", "TPE1", "TALB", "TRCK"]
            missing_fields = []
            
            if audio.tags:
                for field in required_fields:
                    if not audio.tags.get(field):
                        missing_fields.append(field)
            else:
                missing_fields = required_fields
            
            # Check cover art (WAV might have APIC frame)
            cover_art_present = False
            if audio.tags:
                cover_art_present = bool(audio.tags.get("APIC"))
            
            # Check lyrics (WAV uses USLT frame)
            lyrics_present = False
            if audio.tags:
                lyrics_present = bool(audio.tags.get("USLT"))
            
            # Check enhanced metadata
            enhanced_fields = ["TENC", "TSO2", "TSOA"]
            enhanced_present = False
            if audio.tags:
                enhanced_present = all(audio.tags.get(field) for field in enhanced_fields)
            
            validation_result = {
                "success": len(missing_fields) == 0,
                "missing_required_fields": missing_fields,
                "cover_art_present": cover_art_present,
                "lyrics_present": lyrics_present,
                "enhanced_metadata_present": enhanced_present,
                "total_id3_tags": len(audio.tags) if audio.tags else 0,
                "file_size_mb": round(file_path.stat().st_size / (1024 * 1024), 2)
            }
            
            if missing_fields:
                logger.warning(f"Missing WAV metadata fields: {missing_fields}")
            
            if not cover_art_present:
                logger.info("No cover art found in WAV file (external cover art may be available)")
            
            if not lyrics_present:
                logger.info("No lyrics found in WAV file")
            
            if not enhanced_present:
                logger.info("Enhanced metadata not fully present")
            
            return validation_result
            
        except Exception as e:
            logger.error(f"Error validating WAV metadata: {e}")
            return {"error": str(e)}

    def validate_ogg_metadata(self, file_path: Path) -> Dict[str, Any]:
        """Validate OGG metadata after embedding"""
        try:
            if not MUTAGEN_AVAILABLE:
                return {"error": "mutagen not available - cannot validate metadata"}
                
            if not file_path.exists():
                return {"error": "File does not exist"}
            
            file_ext = file_path.suffix.lower()
            if file_ext != ".ogg":
                return {"error": "File is not an OGG file"}
            
            from mutagen.oggvorbis import OggVorbis
            
            audio = OggVorbis(str(file_path))
            
            # Check required fields
            required_fields = ["TITLE", "ARTIST", "ALBUM", "TRACKNUMBER"]
            missing_fields = []
            
            for field in required_fields:
                if not audio.get(field):
                    missing_fields.append(field)
            
            # Check cover art (OGG uses Vorbis comments)
            cover_art_present = bool(audio.get("METADATA_BLOCK_PICTURE") or audio.get("COVERART"))
            
            # Check lyrics (OGG uses Vorbis comments)
            lyrics_present = bool(audio.get("LYRICS") or audio.get("UNSYNCEDLYRICS"))
            
            # Check enhanced metadata
            enhanced_fields = ["ORGANIZATION", "ENCODEDBY", "ENCODING", "SOURCEMEDIA"]
            enhanced_present = all(audio.get(field) for field in enhanced_fields)
            
            validation_result = {
                "success": len(missing_fields) == 0,
                "missing_required_fields": missing_fields,
                "cover_art_present": cover_art_present,
                "lyrics_present": lyrics_present,
                "enhanced_metadata_present": enhanced_present,
                "total_vorbis_comments": len(audio),
                "file_size_mb": round(file_path.stat().st_size / (1024 * 1024), 2)
            }
            
            if missing_fields:
                logger.warning(f"Missing OGG metadata fields: {missing_fields}")
            
            if not cover_art_present:
                logger.info("No cover art found in OGG file")
            
            if not lyrics_present:
                logger.info("No lyrics found in OGG file")
            
            if not enhanced_present:
                logger.info("Enhanced metadata not fully present")
            
            return validation_result
            
        except Exception as e:
            logger.error(f"Error validating OGG metadata: {e}")
            return {"error": str(e)}

    def embed_lyrics(self, request: Dict[str, Any]) -> Dict[str, Any]:
        """Embed lyrics using mutagen and create LRC file for FLAC/WAV"""
        try:
            if not MUTAGEN_AVAILABLE:
                return {"error": "mutagen not available - cannot embed lyrics"}
                
            file_path = Path(request["file_path"])
            lyrics = request["lyrics"]
            
            logger.info(f"Embedding lyrics into: {file_path}")
            logger.info(f"File path exists: {file_path.exists()}")
            logger.info(f"File path absolute: {file_path.absolute()}")
            logger.info(f"File path as string: {str(file_path)}")
            
            if not file_path.exists():
                logger.error(f"File does not exist: {file_path}")
                return {"error": "File does not exist"}
            
            # Format lyrics with proper line breaks
            formatted_lyrics = self._format_lyrics(lyrics)
            
            # Import mutagen based on file extension
            file_ext = file_path.suffix.lower()
            
            if file_ext == ".mp3":
                from mutagen.id3 import ID3, USLT
                from mutagen.mp3 import MP3
                
                audio = MP3(str(file_path), ID3=ID3)
                audio.tags.add(USLT(encoding=3, lang="eng", desc="", text=formatted_lyrics))
                audio.save()
                
            elif file_ext in [".m4a", ".mp4"]:
                from mutagen.mp4 import MP4
                from mutagen.mp3 import MP3
                from mutagen.id3 import ID3, USLT
                
                logger.info(f"Attempting to load M4A file for lyrics: {file_path}")
                try:
                    audio = MP4(str(file_path))
                    logger.info(f"Successfully loaded M4A file for lyrics: {file_path}")
                except Exception as e:
                    logger.warning(f"Failed to load as M4A file for lyrics {file_path}: {e}")
                    logger.info(f"Trying to load as MP3 file for lyrics instead...")
                    try:
                        # Try loading as MP3 if M4A fails
                        audio = MP3(str(file_path), ID3=ID3)
                        logger.info(f"Successfully loaded as MP3 file for lyrics: {file_path}")
                        # Use MP3 format for this file
                        file_ext = ".mp3"
                    except Exception as e2:
                        logger.error(f"Failed to load as both M4A and MP3 for lyrics: {e2}")
                        return {"error": f"Failed to load audio file for lyrics: {e}"}
                
                if file_ext == ".mp3":
                    # Handle as MP3 file
                    audio.tags.add(USLT(encoding=3, lang="eng", desc="", text=formatted_lyrics))
                    audio.save()
                else:
                    # Handle as M4A file
                    audio["\xa9lyr"] = formatted_lyrics
                    audio.save()
                
            elif file_ext == ".flac":
                # For FLAC, embed lyrics in Vorbis comments AND create LRC file
                from mutagen.flac import FLAC
                
                audio = FLAC(str(file_path))
                
                # Embed lyrics in Vorbis comments (most compatible)
                audio["LYRICS"] = formatted_lyrics
                audio["UNSYNCEDLYRICS"] = formatted_lyrics
                
                # Add metadata about lyrics
                audio["LYRICS_LANGUAGE"] = "eng"
                audio["LYRICS_TYPE"] = "unsynced"
                
                audio.save()
                
                # Also create LRC file for compatibility with players that don't read Vorbis comments
                self._create_lrc_file(file_path, formatted_lyrics)
                
            elif file_ext == ".wav":
                # For WAV, embed lyrics using ID3v2 USLT frame AND create LRC file
                from mutagen.id3 import ID3, USLT
                from mutagen.wave import WAVE
                
                audio = WAVE(str(file_path))
                if not audio.tags:
                    audio.add_tags()
                
                # Embed lyrics using USLT frame (same as MP3)
                audio.tags.add(USLT(encoding=3, lang="eng", desc="", text=formatted_lyrics))
                audio.save()
                
                # Also create LRC file for compatibility
                self._create_lrc_file(file_path, formatted_lyrics)
                
            elif file_ext == ".ogg":
                # For OGG, embed lyrics in Vorbis comments AND create LRC file (both Vorbis and FLAC)
                from mutagen.oggvorbis import OggVorbis
                from mutagen.oggflac import OggFLAC
                
                # Try to detect if it's OGG FLAC or OGG Vorbis
                try:
                    audio = OggFLAC(str(file_path))
                    logger.info("Using OggFLAC for lossless OGG lyrics")
                except:
                    audio = OggVorbis(str(file_path))
                    logger.info("Using OggVorbis for lossy OGG lyrics")
                
                # Embed lyrics in Vorbis comments (most compatible)
                audio["LYRICS"] = formatted_lyrics
                audio["UNSYNCEDLYRICS"] = formatted_lyrics
                
                # Add metadata about lyrics
                audio["LYRICS_LANGUAGE"] = "eng"
                audio["LYRICS_TYPE"] = "unsynced"
                
                audio.save()
                
                # Also create LRC file for compatibility
                self._create_lrc_file(file_path, formatted_lyrics)
                
            elif file_ext == ".opus":
                # For Opus, embed lyrics in Vorbis comments AND create LRC file
                from mutagen.opus import Opus
                
                audio = Opus(str(file_path))
                
                # Embed lyrics in Vorbis comments (most compatible)
                audio["LYRICS"] = formatted_lyrics
                audio["UNSYNCEDLYRICS"] = formatted_lyrics
                
                # Add metadata about lyrics
                audio["LYRICS_LANGUAGE"] = "eng"
                audio["LYRICS_TYPE"] = "unsynced"
                
                audio.save()
                
                # Also create LRC file for compatibility
                self._create_lrc_file(file_path, formatted_lyrics)
                
            elif file_ext == ".ape":
                # For APE, embed lyrics in APEv2 tags AND create LRC file
                from mutagen.apev2 import APEv2
                
                audio = APEv2(str(file_path))
                
                # Embed lyrics in APEv2 tags
                audio["Lyrics"] = formatted_lyrics
                audio["Unsynchronised Lyrics"] = formatted_lyrics
                
                # Add metadata about lyrics
                audio["Lyrics Language"] = "eng"
                audio["Lyrics Type"] = "unsynced"
                
                audio.save()
                
                # Also create LRC file for compatibility
                self._create_lrc_file(file_path, formatted_lyrics)
                
            return {"success": True}
            
        except Exception as e:
            logger.error(f"Error embedding lyrics: {e}")
            return {"error": str(e)}

    def read_metadata(self, request: Dict[str, Any]) -> Dict[str, Any]:
        """Read metadata from audio file"""
        try:
            if not MUTAGEN_AVAILABLE:
                return {"error": "mutagen not available - cannot read metadata"}
                
            file_path = Path(request["file_path"])
            
            if not file_path.exists():
                return {"error": "File does not exist"}
            
            # Import mutagen based on file extension
            file_ext = file_path.suffix.lower()
            metadata = {}
            
            if file_ext == ".mp3":
                from mutagen.mp3 import MP3
                
                audio = MP3(str(file_path))
                if audio.tags:
                    metadata = {
                        "title": str(audio.tags.get("TIT2", [""])[0]) if audio.tags.get("TIT2") else "",
                        "artist": str(audio.tags.get("TPE1", [""])[0]) if audio.tags.get("TPE1") else "",
                        "album": str(audio.tags.get("TALB", [""])[0]) if audio.tags.get("TALB") else "",
                        "year": int(audio.tags.get("TYER", [0])[0]) if audio.tags.get("TYER") else None,
                        "genre": str(audio.tags.get("TCON", [""])[0]) if audio.tags.get("TCON") else "",
                        "track_number": int(audio.tags.get("TRCK", [0])[0]) if audio.tags.get("TRCK") else None,
                        "disc_number": int(audio.tags.get("TPOS", [0])[0]) if audio.tags.get("TPOS") else None,
                        "album_artist": str(audio.tags.get("TPE2", [""])[0]) if audio.tags.get("TPE2") else "",
                        "composer": str(audio.tags.get("TCOM", [""])[0]) if audio.tags.get("TCOM") else "",
                        "isrc": str(audio.tags.get("TSRC", [""])[0]) if audio.tags.get("TSRC") else "",
                    }
                    
            elif file_ext in [".m4a", ".mp4"]:
                from mutagen.mp4 import MP4
                
                audio = MP4(str(file_path))
                metadata = {
                    "title": str(audio.get("\xa9nam", [""])[0]) if audio.get("\xa9nam") else "",
                    "artist": str(audio.get("\xa9ART", [""])[0]) if audio.get("\xa9ART") else "",
                    "album": str(audio.get("\xa9alb", [""])[0]) if audio.get("\xa9alb") else "",
                    "year": int(audio.get("\xa9day", [0])[0]) if audio.get("\xa9day") else None,
                    "genre": str(audio.get("\xa9gen", [""])[0]) if audio.get("\xa9gen") else "",
                    "track_number": int(audio.get("trkn", [(0, 0)])[0][0]) if audio.get("trkn") else None,
                    "disc_number": int(audio.get("disk", [(0, 0)])[0][0]) if audio.get("disk") else None,
                    "album_artist": str(audio.get("aART", [""])[0]) if audio.get("aART") else "",
                    "composer": str(audio.get("\xa9wrt", [""])[0]) if audio.get("\xa9wrt") else "",
                    "isrc": str(audio.get("----:com.apple.iTunes:ISRC", [""])[0]) if audio.get("----:com.apple.iTunes:ISRC") else "",
                }
                
            elif file_ext == ".flac":
                
                audio = FLAC(str(file_path))
                metadata = {
                    "title": str(audio.get("TITLE", [""])[0]) if audio.get("TITLE") else "",
                    "artist": str(audio.get("ARTIST", [""])[0]) if audio.get("ARTIST") else "",
                    "album": str(audio.get("ALBUM", [""])[0]) if audio.get("ALBUM") else "",
                    "year": int(audio.get("DATE", [0])[0]) if audio.get("DATE") else None,
                    "genre": str(audio.get("GENRE", [""])[0]) if audio.get("GENRE") else "",
                    "track_number": int(audio.get("TRACKNUMBER", [0])[0]) if audio.get("TRACKNUMBER") else None,
                    "disc_number": int(audio.get("DISCNUMBER", [0])[0]) if audio.get("DISCNUMBER") else None,
                    "album_artist": str(audio.get("ALBUMARTIST", [""])[0]) if audio.get("ALBUMARTIST") else "",
                    "composer": str(audio.get("COMPOSER", [""])[0]) if audio.get("COMPOSER") else "",
                    "isrc": str(audio.get("ISRC", [""])[0]) if audio.get("ISRC") else "",
                }
                
            elif file_ext == ".wav":
                from mutagen.wave import WAVE
                
                audio = WAVE(str(file_path))
                if audio.tags:
                    metadata = {
                        "title": str(audio.tags.get("TIT2", [""])[0]) if audio.tags.get("TIT2") else "",
                        "artist": str(audio.tags.get("TPE1", [""])[0]) if audio.tags.get("TPE1") else "",
                        "album": str(audio.tags.get("TALB", [""])[0]) if audio.tags.get("TALB") else "",
                        "year": int(audio.tags.get("TYER", [0])[0]) if audio.tags.get("TYER") else None,
                        "genre": str(audio.tags.get("TCON", [""])[0]) if audio.tags.get("TCON") else "",
                        "track_number": int(audio.tags.get("TRCK", [0])[0]) if audio.tags.get("TRCK") else None,
                        "disc_number": int(audio.tags.get("TPOS", [0])[0]) if audio.tags.get("TPOS") else None,
                        "album_artist": str(audio.tags.get("TPE2", [""])[0]) if audio.tags.get("TPE2") else "",
                        "composer": str(audio.tags.get("TCOM", [""])[0]) if audio.tags.get("TCOM") else "",
                        "isrc": str(audio.tags.get("TSRC", [""])[0]) if audio.tags.get("TSRC") else "",
                    }
                else:
                    metadata = {}
                    
            elif file_ext == ".ogg":
                from mutagen.oggvorbis import OggVorbis
                from mutagen.oggflac import OggFLAC
                
                # Try to detect if it's OGG FLAC or OGG Vorbis
                try:
                    audio = OggFLAC(str(file_path))
                    logger.info("Using OggFLAC for lossless OGG metadata reading")
                except:
                    audio = OggVorbis(str(file_path))
                    logger.info("Using OggVorbis for lossy OGG metadata reading")
                metadata = {
                    "title": str(audio.get("TITLE", [""])[0]) if audio.get("TITLE") else "",
                    "artist": str(audio.get("ARTIST", [""])[0]) if audio.get("ARTIST") else "",
                    "album": str(audio.get("ALBUM", [""])[0]) if audio.get("ALBUM") else "",
                    "year": int(audio.get("DATE", [0])[0]) if audio.get("DATE") else None,
                    "genre": str(audio.get("GENRE", [""])[0]) if audio.get("GENRE") else "",
                    "track_number": int(audio.get("TRACKNUMBER", [0])[0]) if audio.get("TRACKNUMBER") else None,
                    "disc_number": int(audio.get("DISCNUMBER", [0])[0]) if audio.get("DISCNUMBER") else None,
                    "album_artist": str(audio.get("ALBUMARTIST", [""])[0]) if audio.get("ALBUMARTIST") else "",
                    "composer": str(audio.get("COMPOSER", [""])[0]) if audio.get("COMPOSER") else "",
                    "isrc": str(audio.get("ISRC", [""])[0]) if audio.get("ISRC") else "",
                }
                
            elif file_ext == ".opus":
                from mutagen.opus import Opus
                
                audio = Opus(str(file_path))
                metadata = {
                    "title": str(audio.get("TITLE", [""])[0]) if audio.get("TITLE") else "",
                    "artist": str(audio.get("ARTIST", [""])[0]) if audio.get("ARTIST") else "",
                    "album": str(audio.get("ALBUM", [""])[0]) if audio.get("ALBUM") else "",
                    "year": int(audio.get("DATE", [0])[0]) if audio.get("DATE") else None,
                    "genre": str(audio.get("GENRE", [""])[0]) if audio.get("GENRE") else "",
                    "track_number": int(audio.get("TRACKNUMBER", [0])[0]) if audio.get("TRACKNUMBER") else None,
                    "disc_number": int(audio.get("DISCNUMBER", [0])[0]) if audio.get("DISCNUMBER") else None,
                    "album_artist": str(audio.get("ALBUMARTIST", [""])[0]) if audio.get("ALBUMARTIST") else "",
                    "composer": str(audio.get("COMPOSER", [""])[0]) if audio.get("COMPOSER") else "",
                    "isrc": str(audio.get("ISRC", [""])[0]) if audio.get("ISRC") else "",
                }
                
            elif file_ext == ".ape":
                from mutagen.apev2 import APEv2
                
                audio = APEv2(str(file_path))
                metadata = {
                    "title": str(audio.get("Title", [""])[0]) if audio.get("Title") else "",
                    "artist": str(audio.get("Artist", [""])[0]) if audio.get("Artist") else "",
                    "album": str(audio.get("Album", [""])[0]) if audio.get("Album") else "",
                    "year": int(audio.get("Year", [0])[0]) if audio.get("Year") else None,
                    "genre": str(audio.get("Genre", [""])[0]) if audio.get("Genre") else "",
                    "track_number": int(audio.get("Track", [0])[0]) if audio.get("Track") else None,
                    "disc_number": int(audio.get("Disc", [0])[0]) if audio.get("Disc") else None,
                    "album_artist": str(audio.get("Album Artist", [""])[0]) if audio.get("Album Artist") else "",
                    "composer": str(audio.get("Composer", [""])[0]) if audio.get("Composer") else "",
                    "isrc": str(audio.get("ISRC", [""])[0]) if audio.get("ISRC") else "",
                }
            
            return metadata
            
        except Exception as e:
            logger.error(f"Error reading metadata: {e}")
            return {"error": str(e)}
    
    def _write_progress_file(self, progress_file: Path, progress_info: Dict[str, Any]):
        """Write progress information to a file for external tracking"""
        try:
            with open(progress_file, 'w') as f:
                json.dump(progress_info, f, indent=2)
        except Exception as e:
            logger.error(f"Error writing progress file: {e}")
    
    def _sanitize_track_filename(self, artist: str, title: str) -> str:
        """Sanitize track filename in 'Artist - Title' format"""
        import re
        
        def sanitize_part(text: str) -> str:
            # Remove or replace invalid filename characters
            text = re.sub(r'[<>:"/\\|?*]', '_', text)
            # Remove control characters
            text = re.sub(r'[\x00-\x1f\x7f-\x9f]', '', text)
            # Remove extra whitespace
            text = re.sub(r'\s+', ' ', text).strip()
            return text
        
        sanitized_artist = sanitize_part(artist)
        sanitized_title = sanitize_part(title)
        return f"{sanitized_artist} - {sanitized_title}"
    
    def _rank_search_results(self, tracks: List[Dict[str, Any]], query: str) -> List[Dict[str, Any]]:
        """Rank search results using fuzzy matching"""
        try:
            if not FUZZY_AVAILABLE:
                logger.warning("Fuzzy matching not available - returning results as-is")
                return tracks
            
            # Calculate relevance scores for each track
            scored_tracks = []
            query_lower = query.lower().strip()
            
            for track in tracks:
                title = track.get("title", "").lower()
                artist = track.get("artist", "").lower()
                
                # Create search strings for different matching strategies
                search_strings = [
                    f"{artist} {title}",  # Full combination
                    title,  # Title only
                    artist,  # Artist only
                    f"{title} {artist}",  # Reversed combination
                ]
                
                # Calculate fuzzy scores for different strategies
                scores = []
                for search_string in search_strings:
                    if search_string.strip():
                        # Use different fuzzy matching algorithms
                        ratio_score = fuzz.ratio(query_lower, search_string)
                        partial_score = fuzz.partial_ratio(query_lower, search_string)
                        token_sort_score = fuzz.token_sort_ratio(query_lower, search_string)
                        token_set_score = fuzz.token_set_ratio(query_lower, search_string)
                        
                        # Take the best score from all algorithms
                        best_score = max(ratio_score, partial_score, token_sort_score, token_set_score)
                        scores.append(best_score)
                
                # Calculate final relevance score
                if scores:
                    # Weight different search strategies
                    full_match_score = scores[0] if len(scores) > 0 else 0
                    title_score = scores[1] if len(scores) > 1 else 0
                    artist_score = scores[2] if len(scores) > 2 else 0
                    reversed_score = scores[3] if len(scores) > 3 else 0
                    
                    # Weighted combination (title is most important, then full match, then artist)
                    relevance_score = (
                        title_score * 0.4 +
                        full_match_score * 0.3 +
                        artist_score * 0.2 +
                        reversed_score * 0.1
                    )
                else:
                    relevance_score = 0
                
                # Add bonus for exact matches in title or artist
                if query_lower in title:
                    relevance_score += 20
                if query_lower in artist:
                    relevance_score += 15
                
                # Add bonus for tracks that start with the query
                if title.startswith(query_lower):
                    relevance_score += 25
                if artist.startswith(query_lower):
                    relevance_score += 20
                
                # PRIORITY: Add significant bonus for lyrics versions (they're often better quality and perfectly cut)
                if track.get("is_lyrics_version", False):
                    relevance_score += 100  # Highest priority for explicitly marked lyrics versions
                    logger.debug(f"Lyrics version detected (marked): '{title}' - adding highest priority bonus")
                elif "lyrics" in title.lower():
                    relevance_score += 50  # High priority for lyrics versions found in title
                    logger.debug(f"Lyrics version detected (title): '{title}' - adding priority bonus")
                
                # DEPRIORITIZE: Penalize karaoke tracks and covers (they're not original versions)
                karaoke_indicators = ["karaoke", "instrumental", "backing track", "backing", "no vocals", "acapella", "a capella", "instrumental version", "inst", "minus one", "minus-one"]
                cover_indicators = ["cover", "covers", "cover version", "cover song", "tribute", "by", "performed by", "sung by", "rendition", "version by"]
                
                # Check for karaoke tracks
                for indicator in karaoke_indicators:
                    if indicator in title.lower():
                        relevance_score -= 50  # Significant penalty for karaoke tracks
                        logger.debug(f"Karaoke track detected: '{title}' - applying penalty")
                        break  # Only apply penalty once per track
                
                # Check for cover versions
                for indicator in cover_indicators:
                    if indicator in title.lower():
                        relevance_score -= 30  # Penalty for cover versions
                        logger.debug(f"Cover version detected: '{title}' - applying penalty")
                        break  # Only apply penalty once per track
                
                # Add bonus for other quality indicators
                quality_indicators = ["official", "music video", "mv", "hq", "high quality", "4k", "1080p", "720p"]
                for indicator in quality_indicators:
                    if indicator in title.lower():
                        relevance_score += 10
                
                # Add the relevance score to the track
                track["relevance_score"] = round(relevance_score, 2)
                scored_tracks.append(track)
            
            # Sort by relevance score (highest first)
            ranked_tracks = sorted(scored_tracks, key=lambda x: x.get("relevance_score", 0), reverse=True)
            
            logger.info(f"Ranked {len(ranked_tracks)} tracks by relevance")
            if ranked_tracks:
                logger.info(f"Best match: '{ranked_tracks[0].get('title')}' by '{ranked_tracks[0].get('artist')}' (score: {ranked_tracks[0].get('relevance_score')})")
            
            return ranked_tracks
            
        except Exception as e:
            logger.error(f"Error ranking search results: {e}")
            return tracks
    
    def _get_enhanced_metadata(self, artist: str, title: str, album: str = None, year: int = None, genre: str = None, thumbnail_url: str = None) -> Dict[str, Any]:
        """Get enhanced metadata from external sources"""
        try:
            # Start with basic metadata
            metadata = {
                "title": title,
                "artist": artist,
                "album": album,
                "year": year,
                "genre": genre,
                "cover_art_url": thumbnail_url
            }
            
            # Try to get enhanced metadata from external sources
            if artist and title:
                enhanced_metadata = self._search_enhanced_metadata_from_rust(artist, title)
                if enhanced_metadata:
                    # Merge enhanced metadata with basic metadata
                    metadata.update({
                        "title": enhanced_metadata.get("title", title),
                        "artist": enhanced_metadata.get("artist", artist),
                        "album": enhanced_metadata.get("album", album),
                        "year": enhanced_metadata.get("year", year),
                        "genre": enhanced_metadata.get("genre", genre),
                        "track_number": enhanced_metadata.get("track_number"),
                        "disc_number": enhanced_metadata.get("disc_number"),
                        "album_artist": enhanced_metadata.get("album_artist"),
                        "composer": enhanced_metadata.get("composer"),
                        "isrc": enhanced_metadata.get("isrc"),
                        "cover_art_url": enhanced_metadata.get("cover_art_url", thumbnail_url),
                        "lyrics": enhanced_metadata.get("lyrics")
                    })
                    logger.info(f"Enhanced metadata found: {enhanced_metadata.get('title')} by {enhanced_metadata.get('artist')}")
                else:
                    logger.info("No enhanced metadata found, using basic metadata")
            
            return metadata
            
        except Exception as e:
            logger.error(f"Error getting enhanced metadata: {e}")
            return {
                "title": title,
                "artist": artist,
                "album": album,
                "year": year,
                "genre": genre,
                "cover_art_url": thumbnail_url
            }

    def _search_enhanced_metadata_from_rust(self, artist: str, title: str) -> Optional[Dict[str, Any]]:
        """Search for enhanced metadata using Rust backend providers"""
        try:
            logger.info(f"Searching for enhanced metadata: '{artist}' - '{title}'")
            
            # Create a metadata search request
            search_request = {
                "action": "search_enhanced_metadata",
                "artist": artist,
                "title": title
            }
            
            # Call the Rust backend for metadata search
            # This would be implemented as a subprocess call or shared library
            # For now, we'll return None to indicate no enhanced metadata found
            # In a real implementation, this would call the Rust metadata providers
            
            # TODO: Implement actual call to Rust metadata providers
            # This could be done via:
            # 1. HTTP API call to a local server
            # 2. Shared library call
            # 3. Subprocess call to a Rust binary
            # 4. Message passing through a pipe/socket
            
            return None  # No enhanced metadata found for now
            
        except Exception as e:
            logger.error(f"Error searching enhanced metadata: {e}")
            return None
    
    def _search_lyrics(self, artist: str, title: str) -> str:
        """Search for lyrics from external sources"""
        try:
            # TODO: Implement lyrics search from multiple providers
            # For now, return None (no lyrics found)
            return None
            
        except Exception as e:
            logger.error(f"Error searching lyrics: {e}")
            return None
    
    def _save_lyrics_to_file(self, lyrics: str, artist: str, title: str, album: str = None, lyrics_dir: Path = None):
        """Save lyrics to LRC file"""
        try:
            if not lyrics or not lyrics_dir:
                return
            
            # Sanitize filename components
            def sanitize_filename(name: str) -> str:
                return "".join(c for c in name if c.isalnum() or c in (' ', '-', '_')).rstrip()
            
            artist_clean = sanitize_filename(artist)
            title_clean = sanitize_filename(title)
            album_clean = sanitize_filename(album) if album else None
            
            # Create filename: Artist - Title.lrc (or with album: Artist - Album - Title.lrc)
            if album_clean:
                lyrics_filename = f"{artist_clean} - {album_clean} - {title_clean}.lrc"
            else:
                lyrics_filename = f"{artist_clean} - {title_clean}.lrc"
            
            lyrics_path = lyrics_dir / lyrics_filename
            
            # Write lyrics to file
            with open(lyrics_path, 'w', encoding='utf-8') as f:
                f.write(lyrics)
            
            logger.info(f"Lyrics saved to: {lyrics_path}")
            
        except Exception as e:
            logger.error(f"Error saving lyrics: {e}")
    
    def _download_cover_art(self, thumbnail_url: str, artist: str, title: str, album: str = None) -> Dict[str, Any]:
        """Download cover art from URL or search for it"""
        try:
            if thumbnail_url:
                # Download from provided URL
                import requests
                response = requests.get(thumbnail_url, timeout=10)
                if response.status_code == 200:
                    return {
                        "data": response.content,
                        "mime_type": response.headers.get('content-type', 'image/jpeg'),
                        "url": thumbnail_url
                    }
            
            # TODO: Search for cover art from external sources
            # For now, return None (no cover art found)
            return None
            
        except Exception as e:
            logger.error(f"Error downloading cover art: {e}")
            return None
    
    def _embed_cover_art(self, file_path: str, cover_art: Dict[str, Any]):
        """Embed cover art into audio file"""
        try:
            if not cover_art or not MUTAGEN_AVAILABLE:
                return
            
            file_ext = Path(file_path).suffix.lower()
            
            if file_ext == ".mp3":
                from mutagen.id3 import ID3, APIC
                from mutagen.mp3 import MP3
                
                audio = MP3(file_path, ID3=ID3)
                if not audio.tags:
                    audio.add_tags()
                
                # Add cover art
                audio.tags.add(APIC(
                    encoding=3,  # UTF-8
                    mime=cover_art.get("mime_type", "image/jpeg"),
                    type=3,  # Cover (front)
                    desc="Cover",
                    data=cover_art["data"]
                ))
                
                audio.save()
                
            elif file_ext in [".m4a", ".mp4"]:
                from mutagen.mp4 import MP4
                
                audio = MP4(file_path)
                audio["covr"] = [cover_art["data"]]
                audio.save()
                
            elif file_ext == ".flac":
                from mutagen.flac import FLAC
                
                audio = FLAC(file_path)
                audio["METADATA_BLOCK_PICTURE"] = [cover_art["data"]]
                audio.save()
            
            logger.info(f"Cover art embedded in: {file_path}")
            
        except Exception as e:
            logger.error(f"Error embedding cover art: {e}")

    def validate_opus_metadata(self, file_path: Path) -> Dict[str, Any]:
        """Validate Opus metadata after embedding"""
        try:
            from mutagen.opus import Opus
            
            audio = Opus(str(file_path))
            
            # Check required fields
            required_fields = ["TITLE", "ARTIST", "ALBUM"]
            missing_fields = []
            
            for field in required_fields:
                if not audio.get(field):
                    missing_fields.append(field)
            
            # Check for cover art
            cover_art_present = bool(audio.get("METADATA_BLOCK_PICTURE") or audio.get("COVERART"))
            
            # Check for lyrics
            lyrics_present = bool(audio.get("LYRICS") or audio.get("UNSYNCEDLYRICS"))
            
            # Count Vorbis comments
            total_vorbis_comments = len(audio)
            
            # Get file size
            file_size_mb = file_path.stat().st_size / (1024 * 1024)
            
            validation_result = {
                "success": len(missing_fields) == 0,
                "missing_required_fields": missing_fields,
                "cover_art_present": cover_art_present,
                "lyrics_present": lyrics_present,
                "total_vorbis_comments": total_vorbis_comments,
                "file_size_mb": round(file_size_mb, 2),
                "validation": {
                    "valid": len(missing_fields) == 0,
                    "format": "Opus",
                    "metadata_fields": len(audio),
                    "has_cover_art": cover_art_present,
                    "has_lyrics": lyrics_present
                }
            }
            
            logger.info(f"Opus validation result: {validation_result}")
            return validation_result
            
        except Exception as e:
            logger.error(f"Error validating Opus metadata: {e}")
            return {
                "success": False,
                "error": str(e),
                "validation": {"valid": False, "format": "Opus"}
            }

    def validate_ape_metadata(self, file_path: Path) -> Dict[str, Any]:
        """Validate APE metadata after embedding"""
        try:
            from mutagen.apev2 import APEv2
            
            audio = APEv2(str(file_path))
            
            # Check required fields
            required_fields = ["Title", "Artist", "Album"]
            missing_fields = []
            
            for field in required_fields:
                if not audio.get(field):
                    missing_fields.append(field)
            
            # Check for cover art
            cover_art_present = bool(audio.get("Cover Art (Front)"))
            
            # Check for lyrics
            lyrics_present = bool(audio.get("Lyrics") or audio.get("Unsynchronised Lyrics"))
            
            # Count APEv2 tags
            total_ape_tags = len(audio)
            
            # Get file size
            file_size_mb = file_path.stat().st_size / (1024 * 1024)
            
            validation_result = {
                "success": len(missing_fields) == 0,
                "missing_required_fields": missing_fields,
                "cover_art_present": cover_art_present,
                "lyrics_present": lyrics_present,
                "total_ape_tags": total_ape_tags,
                "file_size_mb": round(file_size_mb, 2),
                "validation": {
                    "valid": len(missing_fields) == 0,
                    "format": "APE",
                    "metadata_fields": len(audio),
                    "has_cover_art": cover_art_present,
                    "has_lyrics": lyrics_present
                }
            }
            
            logger.info(f"APE validation result: {validation_result}")
            return validation_result
            
        except Exception as e:
            logger.error(f"Error validating APE metadata: {e}")
            return {
                "success": False,
                "error": str(e),
                "validation": {"valid": False, "format": "APE"}
            }

def main():
    """Main entry point for the subprocess"""
    # Set UTF-8 encoding for stdin/stdout
    sys.stdin.reconfigure(encoding='utf-8')
    sys.stdout.reconfigure(encoding='utf-8')
    sys.stderr.reconfigure(encoding='utf-8')
    
    processor = AudioProcessor()
    
    # Read input from stdin
    try:
        input_data = json.loads(sys.stdin.read())
        logger.info(f"Received input: {input_data}")
        logger.info(f"Input bytes: {str(input_data).encode('utf-8')}")
        result = processor.process_request(input_data)
        print(json.dumps(result, ensure_ascii=False))
    except Exception as e:
        logger.error(f"Error in main: {e}")
        error_result = {"error": f"Failed to process request: {e}"}
        print(json.dumps(error_result, ensure_ascii=False))
        sys.exit(1)

if __name__ == "__main__":
    main()
