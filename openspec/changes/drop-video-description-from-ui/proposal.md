# Proposal

## Why

YouTube descriptions are mostly spam, and the detail pane already links to the video on YouTube for anyone who wants the full text. Removing them now cleans up the pane ahead of showing AI summaries there, and doesn't depend on that work.

## What Changes

- The video detail pane stops showing the description, including its "Show more" control and its links.
- The pane's More/Less toggle goes away: the status indicator and storage location are always visible, on mobile too.
- **BREAKING**: the playlist, channel and category video listings stop returning `description`. Only the web UI reads it.
- Unchanged: the description stays stored as `plot` and written to the NFO, so Plex, Kodi and Jellyfin keep it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `web-ui`: the detail pane drops the description and the collapsible section.
- `video-listing`: listed videos no longer include a description.
- `e2e-tests`: the mobile layout coverage checks the details without a "More" control.

## Impact

- Web: `web/src/components/VideoDetail.tsx`, its test, `web/src/api/types.ts`, the `aVideo` builder.
- Backend: `src/application/http/videos/dto.rs` and the handler tests in `src/application/http/videos/mod.rs`.
- E2E: `e2e-tests/tests/mobileWatch.spec.js`.
- Not affected: the database, NFO generation, YouTube metadata fetching.
