# Yarrtube

Download every video in a YouTube playlist to a local directory.

## Setup

1. Copy `.env.example` to `.env` and fill in a YouTube Data API v3 key:
   ```bash
   cp .env.example .env
   # then edit .env and set YOUTUBE_API_KEY
   ```
2. Make sure [`yt-dlp`](https://github.com/yt-dlp/yt-dlp) is installed and available on your `PATH`.
3. Build the release binary:
   ```bash
   cargo build --release
   ```

## Usage

```bash
./target/release/yarrtube <playlist_url> <output_path>
```
1