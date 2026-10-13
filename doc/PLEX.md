# Plex (TV Shows library)

## What you get

Yarrtube lays every tracked playlist and channel out on disk as a **TV
show**, the layout Plex, Kodi and Jellyfin read natively:

```
channels/Some Channel/              <- one show per playlist or channel
├── tvshow.nfo                      <- show title, YouTube id
├── poster.jpg                      <- the channel's avatar (channels only)
├── Season 2025/
│   └── S2025E120101 - A video.{mp4,jpg,nfo}
└── Season 2026/
    ├── S2026E031501 - First video of 15 March.{mp4,jpg,nfo}
    └── S2026E031502 - Second video of 15 March.{mp4,jpg,nfo}

playlists/Some Playlist/
├── tvshow.nfo
└── Season 1/
    ├── S01E001 - First video of the playlist.{mp4,jpg,nfo}
    └── S01E002 - Second video of the playlist.{mp4,jpg,nfo}
```

In Plex this gives you one tile per playlist/channel with a real poster,
16:9 episode thumbnails, Continue Watching, On Deck and "play next
episode", none of which a collection offers.

A channel's episodes are numbered from the video's YouTube publish time:
the season is the publish year, the episode is the month and day followed
by a two-digit index for videos of the channel published that day
(`S2026E031502` is the second video of 15 March 2026).

A playlist is a curated sequence, so its episodes are numbered in playlist
order instead: every playlist is a single `Season 1`, and each video gets
the playlist's next number when yarrtube first sees it in the playlist. The
first sync numbers the whole playlist in its YouTube order (`S01E001`,
`S01E002`, ...); videos added later get the numbers after the last one,
wherever YouTube puts them. A removed video's number is never reused.

Either way a number is assigned once and never changes, so files never
rename and Plex keeps watch state on the right episode. A playlist that is
reordered on YouTube keeps its original numbering in Plex (the web UI
follows the live order).

Nothing below is required to *use* the layout: any player that reads TV
show folders picks it up. The optional part is the **scan trigger**, which
asks Plex to import a new episode as soon as it is downloaded.

## 1. Set up a TV Shows library

Create a **TV Shows** library over the folders holding yarrtube's videos
(for example the host folders mounted at `/videos/playlists` and
`/videos/channels`), and in its _Advanced_ settings:

- **Scanner**: Plex TV Series.
- **Agent**: **Plex NFO Series** (reads `tvshow.nfo` and each episode's
  `.nfo`, matching videos by their YouTube id).
- **Seasons**: _Hide for single-season series_, so a channel that only
  published this year opens straight onto its episodes.
- **Use local assets**: on, so `poster.jpg` and each episode's `.jpg` are
  used.

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

With the scan trigger on, yarrtube asks Plex to scan a show's folder as soon
as one of its videos finishes downloading, in every library whose folders
contain it. You need:

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

The trigger is **off unless all three are set**; with only some of them
yarrtube logs a warning at startup and never contacts Plex. Libraries are
discovered, not configured: yarrtube lists the server's libraries and scans
the show folder in each one whose folders contain it.

`YARRTUBE_PLEX_URL` must be reachable from inside the yarrtube container.
`localhost` there is the container itself, not the machine running Docker,
so `http://localhost:32400` only works if the container uses host
networking. If Plex runs on the same machine, either use the machine's LAN
IP (e.g. `http://192.168.1.10:32400`) or run yarrtube with
`network_mode: host`. With host networking, `ports:` is ignored, so set
`YARRTUBE_PORT` to choose the port the web UI listens on.

A failed scan request never affects the download: it is logged and retried
a few times.

## Troubleshooting

- **A show is missing its poster**: only channels have one (a copy of the
  channel's avatar); playlists have none. Show files are rewritten on every
  sync of their playlist or channel, so a deleted `tvshow.nfo` or
  `poster.jpg` comes back on the next pass.
- **Episodes show as "Episode 31502" without their title**: the library is
  not using the **Plex NFO Series** agent, or _Use local assets_ is off.
- **New episodes don't appear until Plex's next scheduled scan**: check the
  log for `requested Plex scan of downloaded video's folder` after a
  download. If you see `no Plex section contains the folder` instead,
  `YARRTUBE_PLEX_VIDEOS_PATH` doesn't match the folders your TV Shows
  library points at.
- **`error sending request for url (http://.../library/sections)`** in the
  log: yarrtube can't open a connection to Plex at all. A wrong token gives
  a 401 instead. Unless yarrtube uses host networking, check that
  `YARRTUBE_PLEX_URL` isn't `localhost` (see
  [Configure yarrtube](#configure-yarrtube)) and that no firewall on the
  Plex machine blocks port 32400 from Docker's network.
