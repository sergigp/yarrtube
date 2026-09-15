# Yarrtube

<img src="doc/yarr.png" alt="yarr" width="200"/>

Yarrtube watches YouTube playlists you tell it about and automatically
downloads every video in them to a folder on your NAS or computer, using
[`yt-dlp`](https://github.com/yt-dlp/yt-dlp). Point it at a playlist once,
and any new videos added to it later get downloaded on their own.

It runs as a small Docker container meant to stay up permanently alongside
the rest of your media stack (Plex, Jellyfin, etc.).

It can also track a whole YouTube channel, keeping its most recent uploads
(up to a configurable limit) synced the same way.

**Coming soon:**

- A Chrome/Firefox extension to send a video to Yarrtube straight from
  YouTube

## Getting started

### Requirements

- Docker
- A YouTube Data API v3 key — create one for free in the
  [Google Cloud Console](https://console.cloud.google.com/apis/library/youtube.googleapis.com):
  create a project, enable the "YouTube Data API v3", then create an API key
  under **Credentials**.

### Run the container

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

- `-p 8080:8080` exposes the browser UI at `/`, the HTTP API used to track
  playlists under `/api` (and `/status` for a health check).
- `-v /path/on/host/videos:/videos` is where downloaded videos will show up
  on your host — point it at your media library.
- `PUID`/`PGID` are the numeric user/group ID that should own downloaded
  files — see the [Note](#docker-compose) below.

### Track a playlist

```bash
curl -X POST http://<host>:8080/api/playlists \
  -H 'content-type: application/json' \
  -d '{"playlist": "<youtube_playlist_id_or_url>", "name": "My Playlist", "quality": "high"}'
```

`playlist` accepts either a bare YouTube playlist ID or a full YouTube
playlist URL (e.g. `https://www.youtube.com/playlist?list=...` or a
`watch?v=...&list=...` URL). `quality` is `high` (uncapped), `mid` (≤720p), or
`low` (≤480p); downloads always prefer an mp4/h264/aac file for maximum
device/player compatibility. It can't be changed after the playlist is
created.

Yarrtube downloads every existing video in the playlist, then keeps checking
for new ones (every hour by default — see [Configuration](#configuration)).

### Docker Compose

If you run a Docker Compose stack (e.g. on a Synology or other NAS), add:

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
      - /volume1/data/media/youtube:/videos
```

> **Note:** set `PUID`/`PGID` to the numeric user/group ID that should own
> downloaded files on the host (run `id <user>` on the host to find them) —
> the same convention used by `linuxserver.io`/`hotio` images. The container
> starts as root, creates a matching user internally, `chown`s `/videos` and
> its own database, then drops to that user before running anything else. If
> `PUID`/`PGID` are left unset, the container keeps the old behavior of
> running (and writing files) as root. Only the mounted video directory is
> persisted — the container's own database is recreated on restart, which is
> expected. The image bundles a known-good `yt-dlp` binary at build time, so
> a freshly started container always has a usable one even before its
> startup self-update runs.

## Browser UI

Open `http://<host>:8080/` for a small read-only UI showing tracked
playlists, a playlist's videos (click a playlist to drill in), and a tab
listing pending/in-progress tasks. The videos and tasks views poll every 3
seconds, so status changes show up without a manual refresh.

## Managing tracked playlists

The HTTP API (under `/api`) is how you add or remove playlists to track:

| Method   | Path                  | Description                                                |
| -------- | --------------------- | ----------------------------------------------------------- |
| `POST`   | `/api/playlists`      | Track a new YouTube-linked playlist (`{"playlist", "name", "path", "quality"}`); `playlist` is a YouTube playlist ID or URL |
| `GET`    | `/api/playlists`      | List tracked playlists, of either kind                      |
| `DELETE` | `/api/playlists/:id`  | Stop tracking a playlist, of either kind                    |
| `GET`    | `/api/playlists/:id/videos` | List the videos recorded for a playlist                |
| `GET`    | `/api/tasks`          | List tasks that have not yet completed (`pending`/`running`) |

A playlist is either YouTube-linked (backed by an existing YouTube playlist,
its ID validated against the YouTube API) or custom (a caller-supplied UUID
with no YouTube playlist behind it). `GET`/`DELETE /api/playlists` work the
same for both; only creation and video membership differ by kind.

### Custom playlists

A custom playlist has no YouTube playlist behind it — its videos are curated
entirely through this API instead of mirroring an existing YouTube playlist:

| Method   | Path                                          | Description                                                        |
| -------- | ---------------------------------------------- | ------------------------------------------------------------------ |
| `POST`   | `/api/custom-playlists`                        | Create a custom playlist (`{"id", "name", "path", "quality"}`); `id` must be a well-formed, unused UUID |
| `POST`   | `/api/custom-playlists/:id/videos`              | Add a video (`{"video"}`, a YouTube URL or bare video ID); verified and titled via the YouTube API before being accepted |
| `DELETE` | `/api/custom-playlists/:id/videos/:video_id`    | Remove a video from the playlist                                    |

A YouTube-linked playlist's videos can't be added or removed manually —
YouTube stays authoritative for that kind, so membership only ever changes
via reconciliation against the YouTube playlist itself.

## Managing tracked channels

Tracking a channel keeps its `video_limit` most recent uploads synced —
newly-uploaded videos are downloaded automatically, and videos that age out
of that window once newer ones are uploaded are cleaned up automatically:

| Method   | Path                        | Description                                                |
| -------- | --------------------------- | ------------------------------------------------------------ |
| `POST`   | `/api/channels`             | Track a new channel (`{"channel", "quality", "video_limit", "path"}`); `channel` is a YouTube handle (e.g. `@somechannel`) or channel URL |
| `GET`    | `/api/channels`             | List tracked channels                                        |
| `DELETE` | `/api/channels/:handle`     | Stop tracking a channel (also deletes its downloaded files)  |
| `POST`   | `/api/channels/:handle/reconcile` | Run an on-demand reconcile immediately, without affecting the recurring schedule |
| `GET`    | `/api/channels/:handle/videos`    | List the videos recorded for a channel, most recent first |

A channel's `path` works exactly like a playlist's: the storage path for its
downloads, relative to `YARRTUBE_VIDEOS_PATH`.

## Manual commands

These run against an already-running container with `docker exec`, without
restarting it:

```bash
# Force a yt-dlp update (this also runs automatically at container start and hourly while it runs)
docker exec yarrtube yarrtube update-ytdlp
```

## Configuration

Environment variables read by the `serve` daemon (the container's default
command):

| Variable                         | Default                 | Description                                               |
| -------------------------------- | ----------------------- | --------------------------------------------------------- |
| `YOUTUBE_API_KEY`                     | —                       | YouTube Data API v3 key (required)                        |
| `PUID`                                | `0` (root)              | Numeric user ID the daemon runs as and that owns downloaded files |
| `PGID`                                | `0` (root)              | Numeric group ID the daemon runs as and that owns downloaded files |
| `YARRTUBE_PORT`                       | `8080`                  | HTTP port to listen on                                    |
| `YARRTUBE_RECONCILE_INTERVAL_SECONDS` | `3600`                  | How often each tracked playlist or channel is reconciled (membership diff for YouTube-linked playlists and channels, filesystem healing for every playlist) |
| `YARRTUBE_DB_PATH`                    | `yarrtube.sqlite3`      | Path to the internal SQLite file (inside the container)   |
| `YARRTUBE_VIDEOS_PATH`                | `/videos`               | Root directory downloaded videos are saved under (inside the container) |
| `YTDLP_PATH`                          | `/app/bin/yt-dlp`       | Path to the managed `yt-dlp` binary (also the path bundled into the image at build time) |
| `RUST_LOG`                            | `info`                  | Log verbosity (e.g. `RUST_LOG=debug`)                     |

### Reconciliation and the output directory

Each tracked playlist runs a recurring reconcile pass (once at creation, then
every `YARRTUBE_RECONCILE_INTERVAL_SECONDS`) that, in addition to diffing
YouTube membership for a YouTube-linked playlist, always compares the files
actually present in that playlist's output directory against what the
database expects to be there: a `Downloaded` video whose file has gone
missing is reset and redownloaded, and any file that isn't the recorded
download of a currently-downloaded video is deleted as an orphan.

**A playlist's output directory is treated as fully system-owned by this
process.** Any file placed there by hand (or left over from something other
than yarrtube itself) will be deleted on the next reconcile pass, the same as
a genuine orphan. Don't store anything in a tracked playlist's output
directory that you don't want yarrtube to manage.

## Updating

```bash
docker pull ghcr.io/sergigp/yarrtube:latest
docker stop yarrtube && docker rm yarrtube
# then re-run the `docker run` command above (or `docker-compose up -d yarrtube`)
```

## Contributing / development

See [`DEVELOPMENT.md`](DEVELOPMENT.md) for building from source, running
tests, and cutting a release.
