# Changelog

All notable changes to yarrtube are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Also new

- Picking a video in the "Up next" queue starts playing it, like opening a
  video card already did. On iPhone, a video opened from a card may still
  wait for a tap on play.
- On a phone, turning it sideways while a downloaded video is in the player
  fills the screen with the video; turning it back returns to the page
  where you left it, without interrupting playback.

## [1.0.0] - 2026-11-16

AI summaries and categorization, two-way Plex watch sync, chapters, and a
redesigned web UI.

### Highlights

- **AI summaries.** Each video gets a short AI summary, built from its
  subtitles and shown instead of the YouTube description. Turn it on per
  channel or playlist. See [AI](doc/AI.md).
- **AI categorization.** Give a category a prompt ("videos about space")
  and the AI adds matching videos from any channel or playlist. You can
  fix any answer by hand, or ask the AI again for one video, a source or a
  whole category.
- **Run AI locally or on Anthropic.** Use your own Ollama server, or set
  `YARRTUBE_ANTHROPIC_API_KEY` to use Claude: faster, no GPU needed.
- **Plex watch state syncs both ways.** Watched, unwatched and resume
  position are synced between the web UI and Plex.
- **Chapters.** YouTube chapters are embedded in the file, so Plex, Kodi
  and Jellyfin show them, and the web player lists them under the video.
- **A redesigned web UI with dark mode.**

### Also new

- Daily log files in `/config/logs`, kept for 30 days
  (`YARRTUBE_LOGS_PATH`, `YARRTUBE_LOG_RETENTION_DAYS`).
- Subtitles, chapters and quality that YouTube adds late to a new video
  are picked up, without rechecking the whole library forever.
- Choose whether Plex category playlists include watched episodes and how
  they're ordered.
- Open a channel or playlist on YouTube from its page.
- Category videos are ordered by publish date, newest first.
- Mobile player plays inline instead of jumping to fullscreen.
- Clearer "Mark watched" for the current video. "Mark all watched" now
  always asks for confirmation.

### Fixed

- Channels with non-Latin handles get correct folder names and links
  (also in 0.10.1).
- A stuck yt-dlp run is killed after a deadline instead of blocking its
  queue.
- Phone shelves on Home no longer catch vertical swipes.

### Removed

- The `yarrtube migrate-layout` command, which moved 0.8.x libraries to the
  TV-show layout.

## [0.10.0] - 2026-10-30

Categories, subtitles and a redesigned web UI.

### Highlights

- **Categories.** Group channels and playlists by topic ("Science", "Kids")
  and watch each one as a single feed: its own page you can play as one
  queue, "More about Science" rows on Home, and colors throughout the UI.
  See [Categories](doc/CATEGORIES.md).
- **Smart playlists in Plex.** Each category becomes a Plex playlist of
  its unwatched episodes, newest first, across all its shows. Kept
  in sync automatically.
- **Subtitles.** Every download gets its captions as an `.srt` next to it,
  picked up by Plex and playable in the web UI player.
- **A redesigned web UI.** New Home with Continue watching, category rows
  and Quick watches; global search (⌘K / Ctrl+K); watch state on every
  thumbnail; foldable sidebar; a Preferences page to show only the
  sections you want.

### Also new

- Set how often each channel and playlist is checked, from 1 minute to 24
  hours.
- Move a playlist or channel to a new folder, Plex follows it.
- Unwatched counts on playlists.
- The running version shows in the header.

### Fixed

- Downloads now get the quality you chose, instead of sometimes falling
  back to 360p.
- Marking a video watched while it downloads no longer gets lost.

### Upgrade notes

- `YARRTUBE_RECONCILE_INTERVAL_SECONDS` is gone: each channel and playlist
  now has its own interval (3 hours by default), editable in the web UI.
- `:latest` now moves only on stable releases. To get every change as it's
  merged, use `:edge`. See [Image tags](doc/INSTALLATION.md#image-tags).
- API: `PATCH /api/playlists/{id}` no longer requires `exclude_from_home`.

## [0.9.0] - 2026-10-13

Existing libraries need a one-time migration. Read
[UPGRADING-TO-0.9.0.md](https://github.com/sergigp/yarrtube/blob/v0.9.0/doc/UPGRADING-TO-0.9.0.md) before upgrading.

### Breaking

- Channels and playlists are laid out as Plex TV shows instead of movies,
  and `yarrtube migrate-layout` moves an existing library to the new layout.
  See [PLEX.md](doc/PLEX.md#what-you-get). ([33a1416])
- Plex collections support is dropped: each channel and playlist is now its
  own show. `YARRTUBE_PLEX_PLAYLIST_SECTION_ID`,
  `YARRTUBE_PLEX_CHANNEL_SECTION_ID` and
  `YARRTUBE_PLEX_RECONCILE_INTERVAL_SECONDS` are removed. ([33a1416])
- The database and avatars now live under `/config`, which must be mounted
  (see [INSTALLATION.md](doc/INSTALLATION.md#run-the-container)). ([d4b84eb])

### Added

- Edit a channel's video quality and video limit after adding it. ([0561caa],
  @sameerparekh)
- Plex troubleshooting in [PLEX.md](doc/PLEX.md#troubleshooting). ([8263380], @sameerparekh)

### Fixed

- Playback no longer rewinds or switches video when the video gets marked
  watched or the list refreshes. ([87e43dd])
- The container exits on SIGTERM instead of waiting out the stop timeout.
  ([b0a6358], @jason-s13r)
- The image builds with podman. ([491c8ff], @jason-s13r)

[1.0.0]: https://github.com/sergigp/yarrtube/compare/v0.10.0...v1.0.0
[0.10.0]: https://github.com/sergigp/yarrtube/compare/v0.9.0...v0.10.0
[0.9.0]: https://github.com/sergigp/yarrtube/compare/v0.8.0...v0.9.0
[33a1416]: https://github.com/sergigp/yarrtube/commit/33a1416
[d4b84eb]: https://github.com/sergigp/yarrtube/commit/d4b84eb
[0561caa]: https://github.com/sergigp/yarrtube/commit/0561caa
[8263380]: https://github.com/sergigp/yarrtube/commit/8263380
[87e43dd]: https://github.com/sergigp/yarrtube/commit/87e43dd
[b0a6358]: https://github.com/sergigp/yarrtube/commit/b0a6358
[491c8ff]: https://github.com/sergigp/yarrtube/commit/491c8ff
