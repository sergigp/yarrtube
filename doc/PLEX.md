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

Only step 1 is required. Step 2 is optional: it makes new episodes show up
in Plex right after they download and, with the same settings, turns on
step 3, category labels and playlists.

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

## 3. Category labels and playlists

With the scan trigger configured (step 2), yarrtube also mirrors your
categories into Plex, so each category can be watched as
one feed across all its playlists and channels. There is nothing else to
set up.

**Labels.** Every downloaded episode gets one Plex label per category its
video belongs to, named after the category. yarrtube owns the labels of the
episodes in its shows: it writes the full list and removes any other label,
so labels you add to those episodes by hand are removed the next time
yarrtube labels them. An
episode whose video is in no category has no labels. Plex Web shows labels on
movies and shows but not on episodes, so you won't see them there; the
playlists below are what uses them.

**Playlists.** For each category, yarrtube creates a **smart playlist** in
every Plex library holding labelled episodes of it: that library's
**unwatched** episodes of the category, **newest air date first**. A watched
episode leaves the playlist by itself. The playlist is named after the
category, e.g. `Chess`. When a category has episodes in more than one
library, there is one playlist per library, named `Chess · Subscriptions`,
`Chess · Kids`, after each library. The playlists belong to the Plex user
whose token yarrtube uses (`YARRTUBE_PLEX_TOKEN`): other users of your
server, managed or shared, don't get them.

yarrtube keeps them in step with your categories:

- Renaming a category replaces its labels and its playlists.
- Deleting a category, or removing all its playlists and channels, removes
  its labels and deletes its playlists.
- A playlist you delete in Plex is created again on the next sync, while its
  category still has episodes. Delete the category instead to get rid of it.
- Playlists yarrtube didn't create are never changed or deleted, even if
  one has the same name as a category.

**When it syncs.** Labels and playlists are written by a background **Plex
label sync** (_Syncing Plex category labels_ in the Tasks page, **Other**
tab). It runs about 10 seconds after a category changes, about a minute
after each download (giving Plex time to import the episode), and every 15
minutes to catch up on anything missed: episodes Plex hadn't imported yet,
episodes Plex re-imported (e.g. after a show moved), and, after upgrading,
every video already in a category. It runs in its own lane, so a slow Plex
never holds up downloads. If Plex is down, the sync is retried 15 minutes
later; nothing is lost.

**Label restrictions.** If you restrict managed or shared users by label
(Plex's sharing restrictions, which allow or exclude content by label),
the category labels on episodes are labels like any other. Don't use a category
name as a restriction label unless you mean that category's episodes to be
restricted too.

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
- **Category playlists don't show up**: check the Tasks page for a _Syncing
  Plex category labels_ task, and the log for `failed to reach Plex,
  retrying the label sync later`. Episodes are labelled only once Plex has
  imported them, so a playlist appears within 15 minutes of its first
  episode being imported. Playlists are visible only to the Plex user whose
  token yarrtube uses.
- **`error sending request for url (http://.../library/sections)`** in the
  log: yarrtube can't reach Plex. Check that `YARRTUBE_PLEX_URL` isn't
  `localhost` (see [Configure yarrtube](#configure-yarrtube)) and that no
  firewall blocks port 32400.
