# Spotify Downloader Test Suite

This directory contains comprehensive tests for all components of the Spotify Downloader application.

## Test Structure

```
tests/
├── components/
│   ├── rust/                    # Rust component tests
│   │   ├── test_lyrics.rs      # Lyrics search functionality
│   │   ├── test_metadata.rs    # Metadata providers
│   │   └── test_search.rs      # Search functionality
│   ├── python/                 # Python component tests
│   │   ├── test_audio_processor.py    # Audio processing
│   │   └── test_spotify_integration.py # Spotify integration
│   └── integration/            # Integration tests
│       ├── test_end_to_end.py  # Full workflow tests
│       └── test_rust_python_integration.py # Rust-Python integration
├── run_all_tests.py           # Master test runner
└── README.md                  # This file
```

## Running Tests

### Run All Tests
```bash
python tests/run_all_tests.py
```

### Run Individual Test Suites

#### Rust Tests
```bash
cd src-tauri
cargo test -- --nocapture
```

#### Python Component Tests
```bash
python tests/components/python/test_audio_processor.py
python tests/components/python/test_spotify_integration.py
```

#### Integration Tests
```bash
python tests/components/integration/test_end_to_end.py
python tests/components/integration/test_rust_python_integration.py
```

## Test Categories

### 1. Rust Component Tests (`components/rust/`)

- **test_lyrics.rs**: Tests lyrics search functionality
  - Basic lyrics search with well-known songs
  - Proxy support testing
  - Timeout handling
  - Individual provider testing (LRC Lib, Lyrics.ovh, Musixmatch, Genius)

- **test_metadata.rs**: Tests metadata providers
  - Spotify API integration
  - MusicBrainz API integration
  - Deezer API integration
  - Combined metadata search
  - Proxy support testing

- **test_search.rs**: Tests search functionality
  - Basic search operations
  - Deep search functionality
  - Unicode character handling
  - Special character handling
  - Timeout testing
  - Error handling

### 2. Python Component Tests (`components/python/`)

- **test_audio_processor.py**: Tests audio processing functionality
  - Search functionality (single and deep search)
  - yt-dlp integration
  - FFmpeg detection and functionality
  - Quality conversion
  - Bitrate conversion
  - Metadata embedding

- **test_spotify_integration.py**: Tests Spotify integration
  - URL parsing and validation
  - API request building
  - Track data parsing
  - Playlist data parsing
  - Album data parsing
  - Error handling

### 3. Integration Tests (`components/integration/`)

- **test_end_to_end.py**: Tests complete workflows
  - Full download workflow (search → download → verify)
  - Metadata embedding workflow
  - Quality conversion workflow
  - Error handling scenarios
  - Proxy functionality
  - Unicode handling

- **test_rust_python_integration.py**: Tests Rust-Python integration
  - Rust binary execution
  - Python script execution
  - FFmpeg integration
  - yt-dlp integration
  - Environment setup
  - File permissions
  - Network connectivity

## Test Requirements

### Prerequisites
- Python 3.8+
- Rust toolchain
- FFmpeg installed and in PATH
- yt-dlp installed and in PATH
- Required Python packages: `mutagen`, `requests`, `pathlib`

### Environment Variables
- `HTTP_PROXY` or `HTTPS_PROXY`: Optional proxy configuration for testing
- `FFMPEG_BINARY`: Optional custom FFmpeg path

## Test Data

Tests use well-known songs and artists to ensure reliable results:
- The Beatles - Hey Jude
- Adele - Hello
- Ed Sheeran - Shape of You
- Taylor Swift - Shake It Off
- Coldplay - Viva La Vida
- And more...

## Debugging Failed Tests

### Common Issues

1. **Network Connectivity**: Some tests require internet access
2. **External Dependencies**: Ensure FFmpeg and yt-dlp are installed
3. **Proxy Configuration**: Some tests may fail behind corporate firewalls
4. **Rate Limiting**: API rate limits may cause intermittent failures

### Debug Mode

Run tests with verbose output:
```bash
python tests/run_all_tests.py 2>&1 | tee test_output.log
```

### Individual Test Debugging

For specific test failures, run individual tests with debug output:
```bash
python -u tests/components/python/test_audio_processor.py
```

## Contributing

When adding new features:
1. Add corresponding tests to the appropriate test file
2. Update this README if adding new test categories
3. Ensure all tests pass before submitting changes
4. Add integration tests for new workflows

## Test Coverage

The test suite covers:
- ✅ Lyrics search (multiple providers)
- ✅ Metadata search (Spotify, MusicBrainz, Deezer)
- ✅ Audio search and download
- ✅ Quality conversion and bitrate handling
- ✅ Spotify integration (URLs, API, parsing)
- ✅ Error handling and edge cases
- ✅ Unicode and special character handling
- ✅ Proxy support
- ✅ Rust-Python communication
- ✅ End-to-end workflows
