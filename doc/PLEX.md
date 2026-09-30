# Plex integration notes

> Draft notes on everything you need to do **manually in Plex** to get the
> collections integration working. Assumes yarrtube itself is already
> installed and downloading videos ([Installation](INSTALLATION.md)).

## What you get

Without the integration, a Plex library over yarrtube's videos is a flat
grid of every downloaded video. With it, yarrtube keeps **one Plex
collection per tracked playlist and channel**: the library becomes one tile
per playlist/channel, videos inside keep their playlist order (publish
order for channels), and autoplay carries you from one video to the next.

Everything below is a one-time setup. Once configured, yarrtube converges
the collections automatically in the background (every 15 minutes by
default): new downloads are added once Plex has scanned them, videos that
leave yarrtube's state are removed, and deleting a playlist/channel in
yarrtube deletes its collection in Plex.

## 1. Give Plex access to yarrtube's videos

Plex and yarrtube must both see the downloaded files:

- Mount the same host folder into both containers — e.g. the folder mounted
  at `/videos` in yarrtube also mounted (read-only is fine) into Plex.
- The mount paths **don't** have to match: yarrtube matches its videos to
  Plex items by YouTube video ID, never by file path.

Create a **Movies**-type library in Plex pointing at that folder (a
dedicated library just for yarrtube content works best — you probably don't
want YouTube videos mixed into your movie library, and the recommended
settings below are per-library).

## 2. Make Plex read yarrtube's metadata files

Next to every video, yarrtube writes a `movie.nfo` file carrying the video's
title, a position-prefixed sort title, and the YouTube video ID
(`<uniqueid type="youtube">...</uniqueid>`). Plex must ingest these NFO
files — they are what gives every item:

- a `youtube://<video-id>` GUID, which is **how the sync recognizes a
  video** (no NFO ingestion → no GUID → the video never joins a collection);
- the sort title that keeps collections in playlist order.

In the library's settings (Manage Library → Edit → Advanced), configure the
agent/scanner to use local metadata: enable **"Use local assets"** and
prefer local metadata over online agents so the NFO wins.
<!-- TODO(sergi): confirm the exact agent/setting names of your working
     setup here — this is the part that varies most between Plex versions. -->

**Verify it worked** before going further: in the Plex web app open any
yarrtube-downloaded video → `⋯` → *Get Info* → *View XML*. You should see
a line like:

```xml
<Guid id="youtube://dQw4w9WgXcQ" />
```

If there is no `youtube://` GUID, fix the library's metadata settings first
(and "Refresh Metadata" on the library) — the sync cannot match anything
without it.

## 3. Get your Plex token (`YARRTUBE_PLEX_TOKEN`)

Follow Plex's official guide:
[Finding an authentication token / X-Plex-Token](https://support.plex.tv/articles/204059436-finding-an-authentication-token-x-plex-token/).

Short version: in the Plex web app, open any library item → `⋯` →
*Get Info* → *View XML*, and copy the `X-Plex-Token=...` value from the
opened page's URL.

## 4. Find the library section ID(s) (`YARRTUBE_PLEX_SECTION_ID`)

Two ways:

- **Browser URL**: open the library in the Plex web app and look at the
  address bar — the number after `source=` is the section ID.
- **API**: list all libraries and pick the `key` of the one(s) holding
  yarrtube content:

  ```bash
  curl -H "Accept: application/json" \
    "http://<YOUR_PLEX_IP>:32400/library/sections?X-Plex-Token=<YOUR_TOKEN>"
  ```

If yarrtube's content is spread across **several Plex libraries** (e.g. one
library per family member), collect every library's section ID — the
variable takes a comma-separated list, and yarrtube keeps each library's
collections in sync independently: a playlist's/channel's collection is
created in whichever listed library its videos were scanned into.
Libraries *not* in the list are never touched.

## 5. Configure yarrtube

Set the environment variables on the yarrtube container and restart it:

```yaml
environment:
  - YARRTUBE_PLEX_URL=http://<YOUR_PLEX_IP>:32400
  - YARRTUBE_PLEX_TOKEN=<YOUR_TOKEN>
  # one section id, or a comma-separated list (e.g. 2,5) when yarrtube's
  # content is spread across several libraries:
  - YARRTUBE_PLEX_SECTION_ID=<YOUR_SECTION_ID>
  # optional, defaults to 900 (15 minutes):
  # - YARRTUBE_PLEX_RECONCILE_INTERVAL_SECONDS=900
```

The integration is **off unless all three are set** — without them yarrtube
behaves exactly as before and never contacts Plex.

On startup the log should show
`scheduled recurring Plex collections reconcile task`; after each pass you
will see `Plex collections reconcile pass succeeded` (or a logged error if
Plex was unreachable — the pass is simply retried on the next interval).

## 6. Recommended library settings in Plex

- **Hide items which are in collections** (the library's Advanced
  settings): this is what turns the library into one tile per
  playlist/channel — videos only appear inside their collection, not
  loose in the grid. Strongly recommended; the integration is built around
  this browsing experience.
- **Don't change a collection's sorting**: yarrtube creates every
  collection with **alphabetical** sorting on purpose — combined with the
  position-prefixed sort titles from the NFO files, that is what keeps
  videos in playlist order. Switching a collection to "Release date" or
  "Custom" breaks the ordering.

## How the sync behaves (what to expect)

- A new video shows up in its collection only after **both** yarrtube has
  downloaded it **and** Plex has scanned it. If it's missing, wait for the
  next scan + sync pass; nothing needs fixing manually. Consider enabling
  Plex's "Scan my library automatically" so new files are picked up quickly.
- A collection is only created once at least one of its videos is scanned —
  you'll never see empty collections.
- Deleting a playlist/channel in yarrtube deletes its Plex collection.
  Deleting a collection by hand in Plex is not permanent: the next sync
  pass recreates it (with its current videos).
- Collections are matched **by name**: the collection's title is the
  playlist/channel name as shown in yarrtube. Renaming a collection in Plex
  will make yarrtube create a fresh one with the original name.
- If Plex is down, yarrtube keeps downloading normally; the sync pass logs
  an error and retries on the next interval.

## Troubleshooting

- **No collections appear at all**: check the startup log for
  `scheduled recurring Plex collections reconcile task` (integration
  enabled?) and for `Plex collections reconcile pass failed` errors (wrong
  URL/token/section id?). Test your values by hand:
  `curl "http://<PLEX>:32400/library/sections/<ID>/all?X-Plex-Token=<TOKEN>"`.
- **A collection is missing videos**: the video isn't downloaded yet, Plex
  hasn't scanned it yet, or its Plex item has no `youtube://` GUID (see the
  verification in step 2).
- **Videos are in the wrong order inside a collection**: the collection's
  sorting was changed away from alphabetical (see step 6), or the items'
  sort titles weren't read from the NFO files.
