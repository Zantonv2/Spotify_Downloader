# 🎵 Spotify Downloader

A modern, cross-platform desktop application built with Tauri and React that allows you to download music from various sources with high-quality metadata and lyrics embedding.

## ✨ Features

### 🎯 Core Functionality
- **Multi-Source Search**: Search across YouTube, SoundCloud, Bandcamp, Vimeo, and more
- **High-Quality Downloads**: Support for MP3, M4A, FLAC, and WAV formats
- **Smart Metadata**: Automatic metadata fetching from Spotify, MusicBrainz, iTunes, and Deezer
- **Lyrics Integration**: Automatic lyrics fetching and embedding from multiple providers
- **Cover Art**: High-quality album artwork embedding
- **Batch Processing**: Import entire Spotify playlists or CSV files

### 🎨 User Interface
- **Modern Design**: Beautiful glassmorphism UI with dark theme
- **Responsive Layout**: Optimized for different screen sizes
- **Real-time Progress**: Live download progress and status updates
- **Multi-Select**: Ctrl+click to select multiple tracks
- **Smart Filtering**: Filter by status (Downloading, Completed, Failed, Pending)
- **Search & Sort**: Find tracks quickly with powerful search

### ⚡ Performance
- **Concurrent Downloads**: Configurable parallel download limits
- **Smart Caching**: Efficient metadata and FFmpeg path caching
- **Background Processing**: Non-blocking UI during downloads
- **Memory Efficient**: Optimized for large playlists (500+ tracks)

## 🚀 Quick Start

### Prerequisites
- **Rust** (latest stable version)
- **Node.js** (v16 or higher)
- **FFmpeg** (for audio processing)
- **Python** (for metadata embedding)

### Installation

1. **Clone the repository**
   ```bash
   git clone https://github.com/ZantonV2/Spotify_Downloader.git
   cd Spotify_Downloader
   ```

2. **Install dependencies**
   ```bash
   # Install Rust dependencies
   cargo build
   
   # Install Node.js dependencies
   npm install
   ```

3. **Configure API Keys**
   - Open the app and go to Settings
   - Add your API keys for:
     - Spotify (Client ID & Secret)
     - Musixmatch (for lyrics)
     - Genius (for lyrics)
     - Deezer (for metadata)

4. **Run the application**
   ```bash
   npm run tauri dev
   ```

## 📖 Usage

### Basic Search & Download
1. Switch to the **Search** tab
2. Enter your search query (song name, artist, album)
3. Select tracks from the results
4. Click "Add to Queue" to start downloading

### Import Playlists
1. Switch to the **Import** tab
2. **Spotify Playlist**: Paste playlist URL and import
3. **CSV File**: Upload a CSV file with track information
4. Tracks will be automatically added to the download queue

### Manage Downloads
1. Switch to the **Downloads** tab
2. View all queued tracks with real-time progress
3. Use filters to find specific tracks
4. Multi-select tracks for batch operations
5. Monitor download statistics and ETA

## ⚙️ Configuration

### Download Settings
- **Format**: Choose between MP3, M4A, FLAC, WAV
- **Quality**: Select bitrate (128-320 kbps for lossy formats)
- **Concurrent Downloads**: Set parallel download limit (1-10)
- **Download Path**: Choose where to save files

### Metadata Settings
- **Enable Metadata**: Toggle automatic metadata embedding
- **Enable Lyrics**: Toggle lyrics fetching and embedding
- **Enable Cover Art**: Toggle album artwork downloading

### API Configuration
- **Spotify**: For playlist imports and basic metadata
- **MusicBrainz**: For detailed metadata (genre, year, etc.)
- **Lyrics Providers**: Musixmatch, Genius for lyrics
- **Deezer**: Additional metadata source

## 🛠️ Development

### Project Structure
```
src-tauri/
├── src/
│   ├── commands.rs          # Tauri commands
│   ├── downloader/          # Download strategies
│   ├── metadata/            # Metadata providers
│   ├── search/              # Search functionality
│   └── utils.rs             # Utility functions
src/
├── components/              # React components
├── hooks/                   # Custom React hooks
├── types/                   # TypeScript definitions
└── App.tsx                  # Main application
```

### Building for Production
```bash
# Build the application
npm run tauri build

# The built application will be in src-tauri/target/release/
```

### Development Commands
```bash
# Start development server
npm run tauri dev

# Build frontend only
npm run build

# Type checking
npm run type-check
```

## 🔧 Technical Details

### Architecture
- **Frontend**: React + TypeScript + Tailwind CSS
- **Backend**: Rust + Tauri
- **Audio Processing**: FFmpeg + Python (mutagen)
- **Search**: yt-dlp with multiple source support
- **Metadata**: Multiple API integrations

### Download Sources
- **Primary**: YouTube (via yt-dlp)
- **Secondary**: SoundCloud, Bandcamp, Vimeo
- **Fallback**: Alternative YouTube sources

### Metadata Providers
- **Spotify**: Basic track info and cover art
- **MusicBrainz**: Detailed metadata (genre, year, album info)
- **iTunes**: Additional metadata source
- **Deezer**: Backup metadata provider

### Lyrics Providers
- **LRC Lib**: Primary lyrics source
- **Lyrics.ovh**: Secondary source
- **Musixmatch**: Premium lyrics (requires API key)
- **Genius**: Additional source (requires API key)

## 📋 Requirements

### System Requirements
- **OS**: Windows 10+, macOS 10.15+, or Linux
- **RAM**: 4GB minimum, 8GB recommended
- **Storage**: 1GB free space
- **Network**: Internet connection for downloads and metadata

### Dependencies
- **FFmpeg**: Required for audio processing
- **Python 3.8+**: Required for metadata embedding
- **pip packages**: mutagen, requests

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **Tauri** for the amazing desktop app framework
- **yt-dlp** for the powerful video/audio downloading
- **FFmpeg** for audio processing
- **All API providers** for metadata and lyrics services

## 📞 Support

If you encounter any issues or have questions:
1. Check the [Issues](https://github.com/ZantonV2/Spotify_Downloader/issues) page
2. Create a new issue with detailed information
3. Join our community discussions

---

**Note**: This application is for personal use only. Please respect copyright laws and terms of service of the platforms you download from.