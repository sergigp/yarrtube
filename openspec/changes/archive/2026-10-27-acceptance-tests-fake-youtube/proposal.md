## Why

A full smoke run takes ~14–16 minutes, about 12 of them downloading real videos, and it depends on YouTube: an API key, the network, and YouTube not rate-limiting or bot-checking the machine (the reason the suite can't run in CI). With YouTube's side covered by the `youtube-contract-tests` capability, the smoke suite can run against a fake YouTube instead, which makes it fast, deterministic and independent of YouTube.

## What Changes

- Three daemon settings, all for running against a fake YouTube: the YouTube Data API base URL (today hardcoded in each adapter), turning the `yt-dlp` self-update off (it would replace a stand-in `yt-dlp` with the real one at startup and every hour), and the background poll interval (with downloads instant, the 5 s between pipeline steps makes each added source take ~20 s to settle). `YTDLP_PATH` is already configurable.
- In `smoke-tests/`: a fake YouTube Data API server (like `plex-stub.js`) and a stand-in `yt-dlp` (run by the Deno the image already ships), both serving one fixture catalogue: playlists, a channel, videos and their metadata, built on a short Creative-Commons clip (`smoke-tests/samples/`), its thumbnail and a subtitle file of our own. Special fixture videos download slowly, fail, or are upcoming premieres.
- Specs add the sources they need themselves through the API (a download takes about a second), so `helpers/seed.js`'s shared seeded downloads go away and no test relies on another's state. The fixture playlist and channel replace the maintainer-owned YouTube playlist and `@BlenderOfficial`.
- New coverage the real YouTube couldn't give reliably: an errored download is surfaced, Sidebar and Tasks search (more than 15 sources), and premieres are skipped until watchable.
- The codec check leaves the smoke suite (it would only check our own fixture file); the YouTube contract tests check real downloads' codecs.
- The upgrade check keeps using the real YouTube, since the released image it starts can't be pointed at the fake until a release ships the API URL setting; it is skipped when no YouTube API key is available.
- No real-YouTube end-to-end run remains: the contract tests cover YouTube's side.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `smoke-tests`: runs against a fake YouTube; tests own their sources; the golden paths use fixture sources; codec coverage moves out; new errored-download, search and premiere coverage.
- `daemon`: a YouTube Data API base-URL setting; a background poll interval setting; the startup self-update can be turned off.
- `ytdlp-self-update`: the automatic updates can be turned off.

## Impact

- `src/serve.rs`, `src/infrastructure/infrastructure_container.rs`, the four `youtube_api_*` adapters (base URL), `src/application/tasks/update_ytdlp_task.rs` (self-update off), `doc/INSTALLATION.md` (the settings, marked as for testing).
- `smoke-tests/`: the fake API, the stand-in `yt-dlp`, the fixture catalogue and files, `helpers/` (seeding, fixtures), every spec using the fixture playlist or channel, `README.md`; `scripts/run-smoke-tests.sh`.
- Smoke runs no longer need `YOUTUBE_API_KEY` except for the upgrade check.
- Follows `youtube-contract-tests` (archived). The rename to acceptance tests comes after this change; CI keeps skipping the smoke suite until the parallel-and-CI change.
