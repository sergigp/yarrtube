## Why

The daemon's contract with YouTube is only exercised indirectly, by the smoke suite downloading real videos: none of the tests in `src/infrastructure/shared/ytdlp.rs` run the real `yt-dlp` (they test argument building and output parsing), and the YouTube Data API repositories are tested against a mock server only. Before the acceptance tests can fake YouTube (`acceptance-tests-fake-youtube`), that contract needs its own tests against the real thing, so a change in YouTube or `yt-dlp` is caught there, and so the fakes have verified behaviour to copy.

## What Changes

- Opt-in infrastructure tests against the real dependencies, marked `#[ignore]` so `cargo test` (and CI) skip them, run by `scripts/run-youtube-contract-tests.sh` (`cargo test -- --ignored youtube_contract`), which needs `yt-dlp`, `ffprobe`, the network and `YOUTUBE_API_KEY`. They use short Creative-Commons or long-stable public videos, credited in the tests.
- `yt-dlp` adapters: downloading a video (an H.264/AAC `.mp4` with its thumbnail, duration and publish time), a missing video's failure, the thumbnail-only fetch, the failure diagnosis, the caption probe and SubRip subtitle fetch, and a channel's newest-videos listing.
- YouTube Data API adapters: resolving a playlist and listing its videos, a video's metadata, resolving a channel handle, and downloading its avatar.
- What these tests assert is what the fake YouTube of `acceptance-tests-fake-youtube` will reproduce; recording fixtures is left to that change.

## Capabilities

### New Capabilities
- `youtube-contract-tests`: what the opt-in contract tests verify against real `yt-dlp` and the YouTube Data API.

### Modified Capabilities
<!-- none -->

## Impact

- `src/infrastructure/repositories/youtube_*_repository.rs` and `filesystem_channel_avatar_repository.rs` (tests only), a shared test helper, `scripts/run-youtube-contract-tests.sh`, `doc/DEVELOPMENT.md`.
- Needs `YOUTUBE_API_KEY` and network to run; ~1 minute.
- Depends on nothing; must land before `acceptance-tests-fake-youtube`.
