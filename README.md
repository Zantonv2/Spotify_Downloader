# Spotify Downloader

A desktop app that lets you download music from Spotify with all the metadata, cover art, and lyrics intact. Built with Rust and React because I wanted something fast and reliable.

## What it does

Basically, you search for music on Spotify and download it to your computer. The app handles all the technical stuff - finding the best audio sources, converting formats, embedding metadata, and making sure everything looks good in your music player.

## Features

### The basics
- Search and download from Spotify
- Multiple audio formats (MP3, M4A, FLAC, WAV, OGG, Opus, APE)
- Download entire playlists or albums
- Queue system so you can download a bunch of stuff at once

### Metadata stuff
- Gets all the track info (title, artist, album, year, etc.)
- Downloads high-res cover art (tries Spotify first, then iTunes, then Cover Art Archive)
- Finds and embeds lyrics
- Makes sure everything is accurate after download

### Performance
- Downloads multiple tracks at the same time
- Retries failed downloads automatically
- Uses your GPU to speed up audio processing
- Caches stuff to make it faster

## Installation

You'll need:
- Rust (latest)
- Node.js (18+)
- Python (3.8+)
- FFmpeg
- yt-dlp

Then just:
```bash
git clone https://github.com/Zantonv2/spotify_downloader.git
cd spotify_downloader
cd src-tauri && cargo build
cd ../python_processor && pip install -r requirements.txt
cd .. && npm install
npm run tauri dev
```

## Audio formats

| Format | Quality | Lossless? | Notes |
|--------|---------|-----------|-------|
| MP3 | 128-320 kbps | No | Most compatible |
| M4A | 128-320 kbps | No | Apple format |
| FLAC | Lossless | Yes | Best quality |
| WAV | Lossless | Yes | Uncompressed |
| OGG | 128-320 kbps | Yes | Open source |
| Opus | 128-320 kbps | No | Efficient |
| APE | Lossless | Yes | High compression |

## How to use

1. Search for music
2. Click "Add to Queue" on tracks you want
3. Go to Downloads tab and click "Download All"
4. Wait for it to finish

You can also import entire playlists from Spotify if you have the playlist URL.

## Settings

There's a settings panel where you can:
- Pick your preferred audio format and quality
- Set how many downloads to run at once
- Choose where to save files
- Enable/disable GPU acceleration
- Turn lyrics on/off

## Troubleshooting

**Downloads not working?**
- Make sure you have FFmpeg installed
- Update yt-dlp: `pip install --upgrade yt-dlp`
- Check your internet connection
- Try reducing the concurrent download limit

**No cover art?**
- Check your internet connection
- The app tries multiple sources, so this is usually a network issue
- Some tracks just don't have cover art available

**Slow downloads?**
- Enable GPU acceleration in settings
- Use an SSD if possible
- Don't set the concurrent limit too high

**Audio quality issues?**
- Make sure FFmpeg is properly installed
- Try a different audio format
- Check if GPU acceleration is working

## Technical details

The app is built with:
- **Rust** for the backend (fast and reliable)
- **React + TypeScript** for the UI
- **Tauri** to put it all together
- **yt-dlp** to find and download audio
- **FFmpeg** to process the audio
- **mutagen** (Python) to handle metadata

## Why I built this

I got tired of other downloaders that either didn't work well, had terrible metadata, or looked awful. So I built my own that does everything right - fast downloads, perfect metadata, beautiful cover art, and a clean interface.

## Legal stuff

This is for personal use only. Don't be a jerk and respect copyright laws. I'm not responsible if you use this for anything sketchy.

## Contributing

Feel free to submit issues or pull requests. The code is pretty straightforward - Rust backend, React frontend, Python for audio processing.

## License

MIT License. Do whatever you want with it.

---

If you find this useful, consider starring the repo. If you find bugs, open an issue. If you want to add features, submit a PR.

Happy downloading! 🎵