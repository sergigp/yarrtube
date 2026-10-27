## Why

The smoke suite runs only in desktop Chromium. Its "mobile" specs just resize that browser to 390px: no touch, no mobile user agent, no WebKit. Mobile (most likely iPhone Safari) is the main way the app is used, yet the watch flow (open a home card → play → resume) is never tested at phone size or in WebKit. Several assertions also pass when the feature is broken (placeholder thumbnails, non-mp4 media, routes that only ever returned 4xx), and the selectors depend on how the DOM happens to be built.

## What Changes

- Add a `mobile-safari` (iPhone, WebKit) Playwright project that runs the specs tagged `@mobile`; desktop Chromium runs everything else. Replace the ad-hoc `setViewportSize` calls with this project. An Android/Chromium project is left out for now to keep run time down.
- New mobile specs:
  - home card → deep-linked video plays and resumes
  - no horizontal overflow on any route
  - tapping a drawer entry navigates and closes the drawer; Escape closes it; the scroll position survives
  - the player sticks under the header
  - video details start collapsed, and "More" expands them
  - add/edit/delete dialogs fit the screen and submit
  - icon-only controls (Tasks tabs, Run now, Sync) keep their accessible names and work
  - the page behind the open drawer doesn't scroll
- Media compatibility checks:
  - downloads are `.mp4` with h264 video and AAC audio (checked with `ffprobe` inside the container)
  - `/media` answers a `Range` request with `206`
  - the video plays in WebKit
- New coverage on any viewport:
  - category detail lists and plays its videos
  - the playback speed menu changes the player's rate
  - an unknown playlist id is reported in the Add playlist dialog
  - reloading a deep link works for every client route, and an unknown source shows an error state
  - the Downloads tab shows an in-progress download
  - deleting a source while its downloads are still running leaves no files behind
  - channel avatars load
- Stricter suite-wide checks:
  - the global fixture also fails on `console.error`, on failed `/media` and `/avatars` requests, and on unexpected 4xx `/api` responses
  - route coverage requires at least one 2xx response per route
  - shared assertions check that images actually loaded and that media is `.mp4` only
- Robustness clean-up:
  - sidebar sections become accessible named lists, and add-dialog submit errors use `role="alert"`; the helpers query those instead of CSS structure and `text-destructive` classes
  - the drawer is detected through the toggle's `aria-expanded` and the sidebar's id
  - `networkidle` waits are removed
  - one sidebar-badge check relies on the periodic refresh instead of a reload
  - the invalid-handle step duplicated in `channel.spec.js` is removed
  - a shared downloaded-playlist fixture (API-seeded) lets the playlist lifecycle mega-test be split into independent tests
- Out of scope, recorded here so they aren't silently dropped:
  - the "errored download is surfaced" test needs a maintainer-owned playlist with an unavailable video
  - Sidebar search and Tasks search need more than 15 real sources or tasks

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `smoke-tests`: adds the mobile browser projects, the mobile and coverage requirements above, media compatibility checks and stricter suite-health checks; replaces the viewport-only "Mobile Sidebar Coverage" requirement.
- `web-ui`: sidebar sections are exposed as lists named after their section, and Add dialog submit errors are announced as alerts.

## Impact

- `smoke-tests/`: `playwright.config.js`, `helpers/*`, `routeCoverage*.js`, every spec under `tests/`, plus new specs.
- `scripts/run-smoke-tests.sh`: install WebKit, export the container name for `ffprobe`.
- `web/src/components/Sidebar.tsx`, `SidebarSectionHeading.tsx`, `AddChannelDialog.tsx`, `AddPlaylistDialog.tsx`, and their Vitest tests (small a11y changes).
- `smoke-tests/README.md`: the "What's covered" section is updated.
- Run time grows (one more project, for the `@mobile` specs only). There are no backend or API changes.
