# Spotify Downloader

A desktop app for downloading music with proper metadata and lyrics. Built with Tauri + React.

## What it does

- Downloads music from YouTube, SoundCloud, and other sources
- Automatically adds metadata (artist, album, year, genre) from Spotify and MusicBrainz
- Embeds lyrics from multiple providers
- Imports entire Spotify playlists or CSV files
- Supports MP3, M4A, FLAC, and WAV formats
- Clean, modern UI with real-time progress tracking

## Why I built this

I wanted a simple way to download music with proper metadata and lyrics. Most downloaders either have terrible UIs or don't handle metadata well. This one does both.

## Getting Started

### Quick Setup (Recommended)

**Windows:** Double-click `setup_wizard.bat` and follow the wizard
**macOS/Linux:** Run `python3 setup_wizard.py` and follow the wizard

### Manual Setup

You'll need:
- Rust
- Node.js 
- FFmpeg
- Python

```bash
git clone https://github.com/ZantonV2/Spotify_Downloader.git
cd Spotify_Downloader
cargo build
npm install
npm run tauri dev
```

See [SETUP.md](SETUP.md) for detailed instructions and troubleshooting.

Add your API keys in Settings (Spotify, Musixmatch, etc.) - the app will work without them but with limited functionality.

## How to use

**Search & Download:**
1. Go to Search tab
2. Type song/artist name
3. Click "Add to Queue"

**Import playlists:**
1. Go to Import tab  
2. Paste Spotify playlist URL or upload CSV
3. Tracks get added to download queue

**Manage downloads:**
1. Go to Downloads tab
2. Watch progress, filter by status
3. Ctrl+click to select multiple tracks

## Settings

- **Download format**: MP3, M4A, FLAC, WAV
- **Quality**: 128-320 kbps
- **Concurrent downloads**: 1-10 parallel downloads
- **Download folder**: Where to save files
- **Metadata**: Auto-fetch from Spotify/MusicBrainz
- **Lyrics**: Auto-fetch and embed
- **Cover art**: Download album artwork

## Development

```bash
npm run tauri dev    # Start dev server
npm run tauri build  # Build for production
```

**Project structure:**
- `src-tauri/src/` - Rust backend (commands, downloader, metadata, search)
- `src/` - React frontend (components, hooks, types)

## Tech Stack

- **Frontend**: React + TypeScript + Tailwind
- **Backend**: Rust + Tauri  
- **Audio**: FFmpeg + Python (mutagen)
- **Search**: yt-dlp (YouTube, SoundCloud, Bandcamp, Vimeo)
- **Metadata**: Spotify, MusicBrainz, iTunes, Deezer
- **Lyrics**: LRC Lib, Lyrics.ovh, Musixmatch, Genius

## Requirements

- Windows 10+, macOS 10.15+, or Linux
- 4GB RAM (8GB recommended)
- FFmpeg for audio processing
- Python 3.8+ for metadata embedding

## Contributing

1. Fork the repo
2. Create a feature branch
3. Make your changes
4. Submit a pull request

## License

MIT License - see [LICENSE](LICENSE) file.

## Issues

Found a bug? Have a feature request? [Open an issue](https://github.com/ZantonV2/Spotify_Downloader/issues).

---

**Note**: This is for personal use only. Please respect copyright laws and platform terms of service.