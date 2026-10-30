# Changelog

All notable changes to yarrtube are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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
[UPGRADING-TO-0.9.0.md](doc/UPGRADING-TO-0.9.0.md) before upgrading.

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

[0.10.0]: https://github.com/sergigp/yarrtube/compare/v0.9.0...v0.10.0
[0.9.0]: https://github.com/sergigp/yarrtube/compare/v0.8.0...v0.9.0
[33a1416]: https://github.com/sergigp/yarrtube/commit/33a1416
[d4b84eb]: https://github.com/sergigp/yarrtube/commit/d4b84eb
[0561caa]: https://github.com/sergigp/yarrtube/commit/0561caa
[8263380]: https://github.com/sergigp/yarrtube/commit/8263380
[87e43dd]: https://github.com/sergigp/yarrtube/commit/87e43dd
[b0a6358]: https://github.com/sergigp/yarrtube/commit/b0a6358
[491c8ff]: https://github.com/sergigp/yarrtube/commit/491c8ff
