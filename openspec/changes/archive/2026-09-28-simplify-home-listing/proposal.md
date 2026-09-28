## Why

Since the home view loads its three sections from `GET /videos/home`, nothing calls `GET /videos/recent`, `GET /videos/continue-watching` or `GET /videos/quick-watches` any more. They are dead API that still costs code, tests and spec. The card read model is also still named after the old recent listing (`RecentVideo`) and wraps the whole `Video` entity instead of exposing the flat fields a card needs (PR #54 review).

## What Changes

- **BREAKING** Remove `GET /videos/recent`, `GET /videos/continue-watching` and `GET /videos/quick-watches`. `GET /videos/home` becomes the only listing of video cards across sources. The web UI and smoke tests already use only `/videos/home`.
- The home listing's requirement carries the rules the removed listings defined:
  - which videos qualify for each section, and in what order
  - one card per YouTube video in the first two sections, the channel copy preferred
  - one card per source under latest videos
  - the fields of each card
- The card read model becomes a flat `HomeVideoView`, built after the section rules have picked the videos. The response DTOs are renamed to match (`HomeVideoResponse`, `HomeVideoSourceResponse`). There is no change to the JSON returned by `/videos/home`.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `video-listing`: remove the recently synced, continue watching and quick watches listings; the home listing requirement states the section rules and card fields itself.

## Impact

- **HTTP API**: three endpoints removed (see above); `/videos/home` unchanged.
- **Domain**: `VideoSearcherApi` keeps `list`, `list_for_channel` and `list_home`; `list_recent`, `list_continue_watching` and `list_quick_watches` are removed. `RecentVideo` becomes `SourcedVideo`, used internally to pair a video with its source; `HomeVideos` holds `HomeVideoView`s.
- **Tests**: the section rule tests move onto the `/videos/home` handler. The `limit` query tests are dropped (home has fixed limits, already covered).
- **Web / smoke tests**: no change; the suite must still pass.
