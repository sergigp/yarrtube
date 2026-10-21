# Plex (TV Shows library)

## What you get

Every tracked playlist and channel shows up in Plex (and Kodi or Jellyfin)
as a **TV show**: one tile per playlist/channel, episode thumbnails,
subtitles, Continue Watching, On Deck and "play next episode".

```
channels/Some Channel/
├── poster.jpg
├── Season 2025/
│   └── S2025E120101 - A video.mp4
└── Season 2026/
    ├── S2026E031501 - First video of 15 March.mp4
    └── S2026E031502 - Second video of 15 March.mp4

playlists/Some Playlist/
└── Season 1/
    ├── S01E001 - First video of the playlist.mp4
    └── S01E002 - Second video of the playlist.mp4
```

- **Channels** get one season per year, with episodes ordered by publish
  date.
- **Playlists** are a single season in playlist order. New videos are added
  at the end.

Episode numbers never change, so Plex keeps your watch progress.

Right after an episode is downloaded, yarrtube also fetches its captions
in the video's original language (uploaded ones, else YouTube's
auto-generated ones; never translations) and saves them next to it as
`S2026E031501 - Title.en.srt`, which Plex picks up as an external subtitle.
The file is named by the base language code (`.es.srt` for `es-US`
captions), which Plex recognises. Plex lists them in the subtitle menu;
whether it turns them on by itself depends on each Plex user's subtitle
settings (_Account → Languages_). Videos without captions in their language
get none, but only after a week of retries: a video is given up on once it
was published at least 7 days ago and its first check without captions was
at least 7 days ago, so captions YouTube hasn't generated yet, and old videos
of a playlist added today, are retried on later syncs. A check where yt-dlp
printed a warning (it can silently withhold captions, e.g. when YouTube
asks for a token) doesn't count, and is retried too. After upgrading, the first sync of each playlist and channel
queues a subtitle fetch for every episode already downloaded; they run one
at a time in the background.

Only step 1 is required. Step 2 is optional and makes new episodes show up
in Plex right after they download.

## 1. Set up a TV Shows library

Create a **TV Shows** library over the folders holding yarrtube's videos
(for example the host folders mounted at `/videos/playlists` and
`/videos/channels`), and in its _Advanced_ settings:

- **Scanner**: Plex TV Series.
- **Agent**: **Plex NFO Series**.
- **Seasons**: _Hide for single-season series_.
- **Use local assets**: on, for posters and thumbnails.

The Plex NFO agents need Plex Media Server 1.43.1 or newer. If **Plex NFO
Series** is missing from the agent list:

- Check the server version under _Settings → General_ and update if it's
  older than 1.43.1.
- After upgrading from a version without the NFO agents, restart Plex Media
  Server once more by hand.
- If it still isn't listed, open the server's own web app at
  `http://<YOUR_PLEX_IP>:32400/web` and create the library there. The
  hosted app at app.plex.tv may not list it yet, even after an upgrade.

Optionally enable _Settings → Library → Scan my library automatically_ and
_Run a partial scan when changes are detected_: Plex then imports new
episodes on its own, and the scan trigger below just makes it immediate.

## 2. Scan trigger (optional)

With the scan trigger on, yarrtube asks Plex to scan a show as soon as one
of its videos finishes downloading, and again once that video's subtitle
file is written. After you
[move a playlist or channel](INSTALLATION.md#moving-a-playlist-or-channel),
it asks Plex to scan both the show's new folder and its old one, so Plex
picks the show up at its new location and drops it from the old. You need:

- A Plex token (`X-Plex-Token`) to authenticate yarrtube's requests.
- The path at which the Plex server sees yarrtube's videos root.

### Get your Plex token (`YARRTUBE_PLEX_TOKEN`)

Follow Plex's official guide:
[Finding an authentication token / X-Plex-Token](https://support.plex.tv/articles/204059436-finding-an-authentication-token-x-plex-token/).

Short version: in the Plex web app, open any library item → `⋯` →
_Get Info_ → _View XML_, and copy the `X-Plex-Token=...` value from the
opened page's URL.

If you can't find _View XML_, or you have a shell on the Plex server, you
can read the token from its `Preferences.xml` instead. On a Linux package
install:

```bash
sudo grep -o 'PlexOnlineToken="[^"]*"' \
  "/var/lib/plexmediaserver/Library/Application Support/Plex Media Server/Preferences.xml"
```

The value between the quotes is the token. Treat it like a password: it
gives full access to your Plex account and server.

### Configure yarrtube

Set the environment variables on the yarrtube container and restart it:

```yaml
environment:
  - YARRTUBE_PLEX_URL=http://<YOUR_PLEX_IP>:32400
  - YARRTUBE_PLEX_TOKEN=<YOUR_TOKEN>
  # the path at which Plex sees yarrtube's videos root, i.e. the host side
  # of the /videos volume (e.g. /volume1/data/media/yarrtube):
  - YARRTUBE_PLEX_VIDEOS_PATH=<HOST_PATH_MOUNTED_AT_/videos>
```

All three are required: with only some of them set, yarrtube logs a
warning and never contacts Plex.

`localhost` inside the container is not your Docker host. If Plex runs on
the same machine, use its LAN IP (e.g. `http://192.168.1.10:32400`) or run
yarrtube with `network_mode: host` (then set `YARRTUBE_PORT` for the web
UI, since `ports:` is ignored).

## Troubleshooting

- **A playlist has no poster**: expected, only channels have one (their
  avatar).
- **Episodes show as "Episode 31502" without their title**: the library is
  not using the **Plex NFO Series** agent, or _Use local assets_ is off.
- **New episodes don't appear until Plex's next scheduled scan**: check the
  log for `requested Plex scan of downloaded video's folder` after a
  download. If you see `no Plex section contains the folder` instead,
  `YARRTUBE_PLEX_VIDEOS_PATH` doesn't match the folders your TV Shows
  library points at.
- **`error sending request for url (http://.../library/sections)`** in the
  log: yarrtube can't reach Plex. Check that `YARRTUBE_PLEX_URL` isn't
  `localhost` (see [Configure yarrtube](#configure-yarrtube)) and that no
  firewall blocks port 32400.
