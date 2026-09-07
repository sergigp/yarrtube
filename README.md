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

## Releasing a new version

Images are built and published automatically by GitHub Actions
(`.github/workflows/release.yml`) — you never need to run `docker build`
or `docker push` by hand. Pushing to `main` only runs the fmt/clippy/build/test
checks (`ci.yml`); the image is only built when you push a tag, so day-to-day
commits don't burn Actions minutes on a Docker build.

To cut a release:

```bash
git tag v0.1.0
git push origin v0.1.0
```

This builds a `linux/amd64` image (matching a typical Synology NAS's CPU) and
pushes it to GitHub Container Registry as both:

- `ghcr.io/sergigp/yarrtube:0.1.0`
- `ghcr.io/sergigp/yarrtube:latest`

**One-time setup:** after the first release, the package is private by
default. Make it public so the NAS can `docker pull` it without
authenticating: on GitHub, go to your profile → **Packages** → `yarrtube` →
**Package settings** → **Change visibility** → **Public**.

## Deploying alongside other services (e.g. a NAS with Docker Compose)

Add a service block like this to your existing `docker-compose.yml`, next to
your other media services:

```yaml
services:
  yarrtube:
    container_name: yarrtube
    mem_limit: 256m
    image: ghcr.io/sergigp/yarrtube:latest
    restart: always
    networks:
      - media
    ports:
      - 8080:8080
    environment:
      - YOUTUBE_API_KEY=your-youtube-data-api-v3-key
      - TZ=Europe/Madrid
    volumes:
      - /volume1/data/media/youtube:/videos
```

Notes:

- **No `PUID`/`PGID`**: unlike the linuxserver/hotio images in the rest of
  the stack, yarrtube's image has no privilege-drop mechanism and runs as
  root. Downloaded files land in the mounted directory owned by root, but
  yt-dlp writes them world-readable, so Plex and other readers are
  unaffected.
- **Volume**: point it at wherever you want downloaded playlists to live
  (adjust the host path to match your library layout). Only this directory
  is persisted — the SQLite file and the yt-dlp binary live inside the
  container's own filesystem and are recreated on each container restart,
  which is expected (see `openspec/specs/daemon/`).
- **Port**: `8080` is `/status`'s default; change the host side
  (`8080:8080` → `<other-port>:8080`) if it collides with something else in
  your stack.

Bring it up and verify:

```bash
docker-compose pull yarrtube
docker-compose up -d yarrtube
curl http://<nas-ip>:8080/status   # expect: 200 OK
```

Run a download or a manual yt-dlp update against the live container:

```bash
docker exec yarrtube yarrtube download "<playlist_id>" /videos
docker exec yarrtube yarrtube update-ytdlp
```

**Updating to a new version:** cut a new tag as above, then on the NAS:

```bash
docker-compose pull yarrtube
docker-compose up -d yarrtube
```

## Subcommands

The image runs `serve` by default. The other subcommands are meant to be run
ad hoc against the already-running container with `docker exec`, without
restarting it:

- `download <playlist_id> <output_path>` - downloads every video in a
  YouTube playlist into `output_path`:
  ```bash
  docker exec yarrtube yarrtube download "<playlist_id>" /videos
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
   ./target/release/yarrtube download <playlist_id> <output_path>
   ./target/release/yarrtube serve
   ./target/release/yarrtube update-ytdlp
   ```
