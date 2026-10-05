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
  -v /path/on/host/config:/config \
  -v /path/on/host/videos:/videos \
  ghcr.io/sergigp/yarrtube:latest
```

- `-v /path/on/host/videos:/videos` is where downloaded videos will show up
  on your host — point it at your media library.
- `-v /path/on/host/config:/config` holds yarrtube's state: the SQLite
  database (your tracked playlists and channels) and channel avatars. Always
  mount it — otherwise everything you've tracked is lost whenever the
  container is recreated, including on every [update](#updating).
- `PUID`/`PGID` are the numeric user/group ID the daemon runs as and that
  owns downloaded files — usually your own user's (`id -u` / `id -g` on the
  host). Leave them unset to run as root.

Then you can open `http://<YOUR_NAS_IP>:8080/` or `http://localhost:8080/`.

> [!WARNING]
> Yarrtube has no authentication: anyone who can reach the port can use the
> web UI and its API, including the folder browser. Keep it on your LAN, or
> put it behind a reverse proxy that handles authentication.

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
      - /path/on/host/config:/config
      - /path/on/host/videos:/videos
```

## Configuration

Environment variables read by Yarrtube are listed below. Every variable except `YOUTUBE_API_KEY` is optional, so you only need to set the ones you want to override:

| Variable                                   | Default               | Description                                                                                                                                                                                                                  |
| ------------------------------------------ | --------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `YOUTUBE_API_KEY`                          | —                     | YouTube Data API v3 key (required)                                                                                                                                                                                           |
| `PUID`                                     | `0` (root)            | Numeric user ID the daemon runs as and that owns downloaded files                                                                                                                                                            |
| `PGID`                                     | `0` (root)            | Numeric group ID the daemon runs as and that owns downloaded files                                                                                                                                                           |
| `YARRTUBE_PORT`                            | `8080`                | HTTP port to listen on                                                                                                                                                                                                       |
| `YARRTUBE_RECONCILE_INTERVAL_SECONDS`      | `3600`                | How often each tracked playlist or channel is reconciled                                                                                                                                                                     |
| `YARRTUBE_DB_PATH`                         | `/config/yarrtube.db` | Path to the SQLite database (inside the container)                                                                                                                                                                           |
| `YARRTUBE_VIDEOS_PATH`                     | `/videos`             | Root directory downloaded videos are saved under (inside the container)                                                                                                                                                      |
| `YARRTUBE_DOWNLOAD_CONCURRENCY`            | `2`                   | How many videos download at the same time. Higher values download faster but make YouTube more likely to throttle or bot-check you                                                                                           |
| `YARRTUBE_RETRY_BASE_DELAY_SECONDS`        | `150`                 | Delay before retrying a failed task; it grows exponentially with each further retry                                                                                                                                          |
| `YARRTUBE_AVATARS_PATH`                    | `/config/avatars`     | Directory channel avatars are stored in (inside the container)                                                                                                                                                               |
| `YTDLP_PATH`                               | `/app/bin/yt-dlp`     | Path to the managed `yt-dlp` binary (also the path bundled into the image at build time)                                                                                                                                     |
| `RUST_LOG`                                 | `info`                | Log verbosity (e.g. `RUST_LOG=debug`)                                                                                                                                                                                        |
| `YARRTUBE_PLEX_URL`                        | —                     | Base URL of your Plex server (e.g. `http://192.168.1.10:32400`). Enables the [Plex collections integration](#plex-collections)                                                                                               |
| `YARRTUBE_PLEX_TOKEN`                      | —                     | Plex authentication token (`X-Plex-Token`)                                                                                                                                                                                   |
| `YARRTUBE_PLEX_PLAYLIST_SECTION_ID`        | —                     | ID(s) of the Plex library section(s) holding yarrtube's **playlist** videos, comma-separated when spread across several libraries (e.g. `2,5`)                                                                               |
| `YARRTUBE_PLEX_CHANNEL_SECTION_ID`         | —                     | ID(s) of the Plex library section(s) holding yarrtube's **channel** videos, comma-separated when spread across several libraries (e.g. `3,6`)                                                                                |
| `YARRTUBE_PLEX_RECONCILE_INTERVAL_SECONDS` | `900`                 | How often Plex collections are synced toward yarrtube's state                                                                                                                                                                |
| `YARRTUBE_PLEX_VIDEOS_PATH`                | —                     | Where the Plex server sees yarrtube's videos root (`YARRTUBE_VIDEOS_PATH`), e.g. the host path `/volume1/data/media/yarrtube` mounted at `/videos`. When set, each downloaded video's folder is scanned into Plex right away |

## Plex collections

Yarrtube can keep one Plex collection per tracked playlist and channel,
turning a flat library of thousands of loose videos into one tile per
playlist/channel, with correct in-playlist ordering and autoplay across
episodes. The integration is off by default: it activates only when
`YARRTUBE_PLEX_URL`, `YARRTUBE_PLEX_TOKEN` and at least one of
`YARRTUBE_PLEX_PLAYLIST_SECTION_ID` / `YARRTUBE_PLEX_CHANNEL_SECTION_ID`
are set. Each Plex library holds a single kind — playlists or channels.

The Plex-side setup (library configuration, obtaining the token and
section IDs, recommended library settings) is covered step by step in
[PLEX.md](PLEX.md).

## Updating

With `docker run`:

```bash
docker pull ghcr.io/sergigp/yarrtube:latest
docker stop yarrtube && docker rm yarrtube
# then re-run the `docker run` command above
```

With Docker Compose:

```bash
docker compose pull yarrtube
docker compose up -d yarrtube
```
