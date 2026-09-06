# Yarrtube

Download every video in a YouTube playlist to a local directory. Packaged as a
Docker image meant to run continuously (e.g. on a NAS), with playlist
downloads and yt-dlp updates run ad hoc against the live container.

## Running the container

1. Copy `.env.example` to `.env` and fill in a YouTube Data API v3 key:
   ```bash
   cp .env.example .env
   # then edit .env and set YOUTUBE_API_KEY
   ```
2. Build the image:
   ```bash
   docker build -t yarrtube .
   ```
3. Start the container. This runs the `serve` daemon by default, which keeps
   running, exposes `GET /status` on the published port, and self-updates
   yt-dlp on startup:
   ```bash
   docker run -d \
     --name yarrtube \
     --env-file .env \
     -p 8080:8080 \
     -v /path/on/host/videos:/videos \
     yarrtube
   ```
   - `-p 8080:8080` publishes the HTTP port so `/status` is reachable at
     `http://<host>:8080/status`.
   - `-v /path/on/host/videos:/videos` mounts a host directory at `/videos`
     so downloaded files are visible outside the container (e.g. to a media
     server like Plex).

## Subcommands

The image runs `serve` by default. The other subcommands are meant to be run
ad hoc against the already-running container with `docker exec`, without
restarting it:

- `download <playlist_url> <output_path>` - downloads every video in a
  YouTube playlist into `output_path`:
  ```bash
  docker exec yarrtube yarrtube download "<playlist_url>" /videos
  ```
- `update-ytdlp` - downloads the latest yt-dlp standalone Linux release and
  replaces the binary the `download` task uses. Runs automatically once each
  time the daemon starts, and can also be run manually:
  ```bash
  docker exec yarrtube yarrtube update-ytdlp
  ```
- `serve` - the long-running daemon (the container's default command). On
  startup it self-updates yt-dlp, checks that its local database is usable,
  then starts the HTTP server and logs a periodic heartbeat.

## Configuration

Environment variables read by `serve`:

- `YARRTUBE_PORT` - HTTP port to listen on (default `8080`).
- `YARRTUBE_DB_PATH` - path to the local SQLite file (default
  `yarrtube.sqlite3`, inside the container's own filesystem; not persisted
  across container recreation in this iteration).
- `YTDLP_PATH` - path to the yt-dlp binary used and updated by `download`,
  `update-ytdlp`, and `serve`'s startup self-update (default
  `/usr/local/bin/yt-dlp`).

## Local development (without Docker)

1. Copy `.env.example` to `.env` and fill in a YouTube Data API v3 key.
2. Make sure [`yt-dlp`](https://github.com/yt-dlp/yt-dlp) is installed and
   available on your `PATH`.
3. Build and run:
   ```bash
   cargo build --release
   ./target/release/yarrtube download <playlist_url> <output_path>
   ./target/release/yarrtube serve
   ./target/release/yarrtube update-ytdlp
   ```
