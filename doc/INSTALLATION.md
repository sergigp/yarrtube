# Installation and configuration

## Requirements

- Docker
- A YouTube Data API v3 key — create one for free in the
  [Google Cloud Console](https://console.cloud.google.com/apis/library/youtube.googleapis.com):
  create a project, enable the "YouTube Data API v3", then create an API key
  under **Credentials**.

## Run the container

```bash
docker run -d \
  --name yarrtube \
  -e YOUTUBE_API_KEY=your-api-key-here \
  -e PUID=1000 \
  -e PGID=1000 \
  -p 8080:8080 \
  -v /path/on/host/videos:/videos \
  ghcr.io/sergigp/yarrtube:latest
```

- `-v /path/on/host/videos:/videos` is where downloaded videos will show up
  on your host — point it at your media library.
- `PUID`/`PGID` are the numeric user/group ID that should own downloaded
  files — see the [Note](#docker-compose) below.

## Docker Compose

If you run a Docker Compose stack, add:

```yaml
services:
  yarrtube:
    container_name: yarrtube
    mem_limit: 256m
    image: ghcr.io/sergigp/yarrtube:latest
    restart: always
    ports:
      - 8080:8080
    environment:
      - YOUTUBE_API_KEY=your-youtube-data-api-v3-key
      - PUID=1000
      - PGID=1000
      - TZ=Europe/Madrid
    volumes:
      - /path/on/host/videos:/videos
```

> **Note:** set `PUID`/`PGID` to the numeric user/group ID that should own
> downloaded files on the host (run `id <user>` on the host to find them) —
> the same convention used by `linuxserver.io`/`hotio` images.

Then you can open `http://<YOUR_NAS_IP>:8080/` or `http://localhost:8080/`.

## Configuration

Environment variables read by the `serve` daemon (the container's default
command):

| Variable                              | Default            | Description                                                                              |
| ------------------------------------- | ------------------ | ---------------------------------------------------------------------------------------- |
| `YOUTUBE_API_KEY`                     | —                  | YouTube Data API v3 key (required)                                                       |
| `PUID`                                | `0` (root)         | Numeric user ID the daemon runs as and that owns downloaded files                        |
| `PGID`                                | `0` (root)         | Numeric group ID the daemon runs as and that owns downloaded files                       |
| `YARRTUBE_PORT`                       | `8080`             | HTTP port to listen on                                                                   |
| `YARRTUBE_RECONCILE_INTERVAL_SECONDS` | `3600`             | How often each tracked playlist or channel is reconciled                                 |
| `YARRTUBE_DB_PATH`                    | `yarrtube.sqlite3` | Path to the internal SQLite file (inside the container)                                  |
| `YARRTUBE_VIDEOS_PATH`                | `/videos`          | Root directory downloaded videos are saved under (inside the container)                  |
| `YTDLP_PATH`                          | `/app/bin/yt-dlp`  | Path to the managed `yt-dlp` binary (also the path bundled into the image at build time) |
| `RUST_LOG`                            | `info`             | Log verbosity (e.g. `RUST_LOG=debug`)                                                    |

## Updating

```bash
docker pull ghcr.io/sergigp/yarrtube:latest
docker stop yarrtube && docker rm yarrtube
# then re-run the `docker run` command above (or `docker-compose up -d yarrtube`)
```
