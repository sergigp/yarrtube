# Tasks

Code and tests for every task below already exist uncommitted on branch `go-to-youtube-buttons`. Applying this change means checking each task against the working tree and fixing any gap.

## 1. Walking skeleton

- [x] 1.1 Every file and signature from design.md (## Files, ## Types & Signatures) exists and is wired: `youtubeChannelUrl`/`youtubePlaylistUrl`, `HeaderAction.href`, the `DetailHeader` `youtubeLink` prop passed by `ChannelDetail`/`PlaylistDetail`, `MarkWatchedButton` replacing `VideoWatchedControl`, and `YouTubeLink` in `VideoDetail`. Verify: `npm run typecheck` and the existing `npm run test` pass.

## 2. Behaviour (TDD)

- [x] 2.1 `sourcePath` tests: the channel URL keeps `@` and encodes non-Latin handles; the playlist URL uses `?list=`. Verify: `npx vitest run src/lib/sourcePath.test.ts`.
- [x] 2.2 `DetailHeader` tests: the source YouTube link is a ⋮ menu item on mobile and an icon link on desktop, both opening in a new tab, labelled by the caller. Verify: `npx vitest run src/components/DetailHeader.test.tsx`.
- [x] 2.3 `ChannelDetail`/`PlaylistDetail` tests: the header menu order includes "Open channel on YouTube" / "Open playlist on YouTube"; the playlist's desktop icon link has the playlist URL; the selected row shows "Actions for …" like every row. Verify: vitest on both files.
- [x] 2.4 `MarkWatchedButton` tests: marks the video and refreshes the library; nothing when watched; nothing when not downloaded; alert on failure; disabled while in flight. Verify: `npx vitest run src/components/MarkWatchedButton.test.tsx`.
- [x] 2.5 `VideoDetail` tests: "Open video on YouTube" in the title row opens in a new tab while collapsed; "More" shows the description and location only; a watched video's meta line starts `Watched · ` with no "Mark watched"; "Mark watched" marks the video. Verify: `npx vitest run src/components/VideoDetail.test.tsx`.
- [x] 2.6 e2e: `channel.spec.js` (mark watched from the pane, "Watched" on the meta line, rewatch brings the button back) and `mobileWatch.spec.js` (YouTube link visible while collapsed, location revealed by "More"). Verify: they pass in the e2e suite.

## 4. Verification

- [x] 4.1 `scripts/run-tests.sh` (no flags) passes: web typecheck, lint and tests; Rust fmt, clippy and tests; contract; e2e.
- [x] 4.2 Manual (agent): start `scripts/run-local.sh --port <free port> --fresh-videos`, seed a channel and a playlist via the HTTP API, then use the Playwright MCP browser. At 1280px, check the title row shows "Mark watched", the speed control and the YouTube icon, and the header shows "Open channel on YouTube" with the right href. At 375px, check the title row shows only the tick, the meta line has no "Synced" and the page doesn't scroll horizontally. Mark the video watched and check the button disappears and the meta line starts with "Watched". Take screenshots. Afterwards stop the server and delete `yarrtube.sqlite3` and the fresh videos dir.
- [ ] 4.3 Manual (human): on a real phone, judge whether the cramped title row is acceptable with long titles (about 145px left for the title in a channel view); check the tick-only button is recognisable; check the YouTube links open in the YouTube app or a new tab as expected.

## Workflow follow-up

- Archive the change (`/opsx:archive`) after review so the deltas fold into `openspec/specs/web-ui` and `openspec/specs/e2e-tests`.
