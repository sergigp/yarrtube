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

### Image tags

| Tag                        | What it is                                                                 |
| -------------------------- | -------------------------------------------------------------------------- |
| `latest`                   | The latest stable release. Recommended.                                    |
| `X.Y.Z` (e.g. `0.9.5`)     | One exact release, to pin a version.                                       |
| `X.Y` (e.g. `0.9`)         | The latest patch release of a minor version.                               |
| `edge`                     | The latest commit on `main`, published on every merge. Unreleased and can break. |
| `sha-<short sha>`          | The image of one specific commit.                                          |

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

| Variable                              | Default               | Description                                                                                                                                                                                                   |
| ------------------------------------- | --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `YOUTUBE_API_KEY`                     | —                     | YouTube Data API v3 key (required)                                                                                                                                                                            |
| `PUID`                                | `0` (root)            | Numeric user ID the daemon runs as and that owns downloaded files                                                                                                                                             |
| `PGID`                                | `0` (root)            | Numeric group ID the daemon runs as and that owns downloaded files                                                                                                                                            |
| `YARRTUBE_PORT`                       | `8080`                | HTTP port to listen on                                                                                                                                                                                        |
| `YARRTUBE_DB_PATH`                    | `/config/yarrtube.db` | Path to the SQLite database (inside the container)                                                                                                                                                            |
| `YARRTUBE_VIDEOS_PATH`                | `/videos`             | Root directory downloaded videos are saved under (inside the container)                                                                                                                                       |
| `YARRTUBE_DOWNLOAD_CONCURRENCY`       | `2`                   | How many videos download at the same time. Higher values download faster but make YouTube more likely to throttle or bot-check you                                                                            |
| `YARRTUBE_RETRY_BASE_DELAY_SECONDS`   | `150`                 | Delay before retrying a failed task; it grows exponentially with each further retry                                                                                                                           |
| `YARRTUBE_AVATARS_PATH`               | `/config/avatars`     | Directory channel avatars are stored in (inside the container)                                                                                                                                                |
| `YTDLP_PATH`                          | `/app/bin/yt-dlp`     | Path to the managed `yt-dlp` binary (also the path bundled into the image at build time)                                                                                                                      |
| `RUST_LOG`                            | `info`                | Log verbosity (e.g. `RUST_LOG=debug`)                                                                                                                                                                         |
| `YARRTUBE_PLEX_URL`                   | —                     | Base URL of your Plex server (e.g. `http://192.168.1.10:32400`). With the token and `YARRTUBE_PLEX_VIDEOS_PATH`, enables the [Plex scan trigger](#plex)                                                       |
| `YARRTUBE_PLEX_TOKEN`                 | —                     | Plex authentication token (`X-Plex-Token`)                                                                                                                                                                    |
| `YARRTUBE_PLEX_VIDEOS_PATH`           | —                     | Where the Plex server sees yarrtube's videos root (`YARRTUBE_VIDEOS_PATH`), e.g. the host path `/volume1/data/media/yarrtube` mounted at `/videos`. Plex is asked to scan a show's folder after each download |

How often each channel and playlist is checked for new videos is not an
environment variable: it is set per source in the web UI ("Check for new
videos every" under "Advanced options" when adding it, or "Edit settings"
afterwards), 3 hours by default.

## Videos on disk

Each playlist and channel is saved as a TV show (layout in
[PLEX.md](PLEX.md#what-you-get)). Keep the videos root on a single
filesystem: downloads are staged in a hidden `.downloads` folder there and
moved into place when complete.

### Moving a playlist or channel

To put a playlist's or channel's videos somewhere else, open its edit
dialog (the source's menu → "Edit settings"), check its current
**Location** and choose **Move…**. Pick the new parent folder under "Save
to" (or "Choose another folder…") and the folder name, then **Move**. Its
show folder is renamed in place: nothing is downloaded again, and every
episode keeps its number and watch progress.

While the source still has downloads or fetches in progress, "Move…" is
disabled with "Downloads in progress — you can move it once they finish."
and becomes available by itself a few seconds after they do.

A move is refused, with nothing changed, while:

- any of its videos is still waiting to download or downloading, or a
  thumbnail, subtitle fetch or file deletion for it is queued or running
  (failed and excluded videos don't count);
- the source is being synced (try again once the sync finishes);
- the new path is the current one, is used by another playlist or channel,
  sits inside one's folder or contains one (its own included);
- a file or folder already exists at the new path.

Moves are renames within the videos root's filesystem: a destination on
another mount is rejected, and the files stay where they were.

## Plex

Point a Plex **TV Shows** library (Plex NFO Series agent) at your videos
folders. Optionally, set `YARRTUBE_PLEX_URL`, `YARRTUBE_PLEX_TOKEN` and
`YARRTUBE_PLEX_VIDEOS_PATH` (all three) and yarrtube asks Plex to scan a
show's folder as soon as one of its videos is downloaded. The library setup
and the scan trigger are covered step by step in [PLEX.md](PLEX.md).

## Updating

> [!NOTE]
> Updating from 0.8.x or older? 0.9.0 moved the library to a TV show layout
> and changed the Plex settings. Follow
> [UPGRADING-TO-0.9.0.md](UPGRADING-TO-0.9.0.md) once instead.

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
