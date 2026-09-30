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

> [!NOTE]
> Set `PUID`/`PGID` to the numeric user/group ID that should own
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
| `YARRTUBE_DOWNLOAD_CONCURRENCY`       | `2`                | How many videos download at the same time. Higher values download faster but make YouTube more likely to throttle or bot-check you |
| `YTDLP_PATH`                          | `/app/bin/yt-dlp`  | Path to the managed `yt-dlp` binary (also the path bundled into the image at build time) |
| `RUST_LOG`                            | `info`             | Log verbosity (e.g. `RUST_LOG=debug`)                                                    |
| `YARRTUBE_PLEX_URL`                   | —                  | Base URL of your Plex server (e.g. `http://192.168.1.10:32400`). Enables the [Plex collections integration](#plex-collections) |
| `YARRTUBE_PLEX_TOKEN`                 | —                  | Plex authentication token (`X-Plex-Token`)                                               |
| `YARRTUBE_PLEX_SECTION_ID`            | —                  | ID of the Plex library section holding yarrtube's videos                                 |
| `YARRTUBE_PLEX_RECONCILE_INTERVAL_SECONDS` | `900`         | How often Plex collections are synced toward yarrtube's state                            |

## Plex collections

Yarrtube can keep one Plex collection per tracked playlist and channel,
turning a flat library of thousands of loose videos into one tile per
playlist/channel, with correct in-playlist ordering and autoplay across
episodes.

The integration is off by default: it activates only when
`YARRTUBE_PLEX_URL`, `YARRTUBE_PLEX_TOKEN` and `YARRTUBE_PLEX_SECTION_ID`
are all set. A recurring background task then converges the collections
toward yarrtube's downloaded videos (creating collections with alphabetical
sorting so videos keep their playlist order, adding videos as Plex scans
them, and removing videos that leave yarrtube's state), and deleting a
playlist or channel deletes its collection.

To configure it:

1. **Token** — follow Plex's guide to
   [find your `X-Plex-Token`](https://support.plex.tv/articles/204059436-finding-an-authentication-token-x-plex-token/)
   (open any library item in the Plex web app, `⋯` → *Get Info* →
   *View XML*, and copy the `X-Plex-Token` value from the URL).
2. **Section ID** — list your libraries and note the `key` of the one
   holding yarrtube's videos:

   ```bash
   curl "http://<YOUR_PLEX_IP>:32400/library/sections?X-Plex-Token=<YOUR_TOKEN>" \
     -H "Accept: application/json" | grep -o '"key":"[0-9]*","title":"[^"]*"'
   ```

3. Set the `YARRTUBE_PLEX_*` variables on the container and restart it.

> [!TIP]
> In the library's settings in Plex, enable **"Hide items which are in
> collections"** (Manage Library → Edit → Advanced). The library then
> shows one tile per playlist/channel instead of every video, which is
> the browsing experience this integration is built for.

## Updating

```bash
docker pull ghcr.io/sergigp/yarrtube:latest
docker stop yarrtube && docker rm yarrtube
# then re-run the `docker run` command above (or `docker-compose up -d yarrtube`)
```
