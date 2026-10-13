# Changelog

All notable changes to yarrtube are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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

[0.9.0]: https://github.com/sergigp/yarrtube/compare/v0.8.0...v0.9.0
[33a1416]: https://github.com/sergigp/yarrtube/commit/33a1416
[d4b84eb]: https://github.com/sergigp/yarrtube/commit/d4b84eb
[0561caa]: https://github.com/sergigp/yarrtube/commit/0561caa
[8263380]: https://github.com/sergigp/yarrtube/commit/8263380
[87e43dd]: https://github.com/sergigp/yarrtube/commit/87e43dd
[b0a6358]: https://github.com/sergigp/yarrtube/commit/b0a6358
[491c8ff]: https://github.com/sergigp/yarrtube/commit/491c8ff
