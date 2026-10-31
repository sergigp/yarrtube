# Tasks

## 1. Page header: "Mark all watched" into the ⋮ menu on desktop

- [x] 1.1 Add `menuOnDesktop` to `HeaderAction` in `web/src/components/DetailHeader.tsx`; on desktop render flagged actions in `MoreActionsMenu` (above the separator and Delete) and the rest as icon buttons; set it on "Mark all watched". Verify: `DetailHeader.test.tsx` desktop test asserts no "Mark all watched" button and that the "⋮" menu lists "Mark all watched" then "Delete"; existing confirm and narrow-viewport tests still pass.
- [x] 1.2 Update the desktop header button order assertions in `PlaylistDetail.test.tsx`, `ChannelDetail.test.tsx` and `CategoryDetail.test.tsx` (button lists no longer include "Mark all watched"; mark-all flows open "More actions for …" first). Verify: `npm run test` in `web/` passes.
- [ ] 1.3 Update `e2e-tests/tests/detailHeader.spec.js` (desktop button list) and `e2e-tests/tests/categories.spec.js:138` (open "More actions for <name>" and choose the "Mark all watched" menu item). Verify: these specs pass in `scripts/run-tests.sh`.

## 2. Video detail pane: labelled watched control

- [x] 2.1 Add `web/src/components/VideoWatchedControl.tsx`: renders nothing unless downloaded; "Mark watched" outline button (marks via `useMarkVideoWatched`, disabled while pending, alerts on failure) when unwatched; muted non-interactive "Watched" with `title="Rewatch past 10% to mark it unwatched"` when watched. Verify: `VideoWatchedControl.test.tsx` covers marking (POST route hit), watched status (no button, hint present), not downloaded (renders nothing) and failure (alert, still "Mark watched").
- [ ] 2.2 In `VideoDetail.tsx`, render `VideoWatchedControl` in its own row below `MetaLine` and remove `VideoActionsMenu` from the title row. Verify: `VideoDetail.test.tsx` replaces the menu-based mark test with the button, and asserts no "Actions for <title>" button in the pane.
- [ ] 2.3 Update `ChannelDetail.test.tsx` (lines ~190–200: the selected video has no "⋮" anywhere; the detail pane shows "Mark watched") and any other test that opened the detail pane's menu. Verify: `npm run check` in `web/` passes.
- [ ] 2.4 Update `e2e-tests/tests/channel.spec.js:113` to mark the selected video via the "Mark watched" button if it targets the detail pane's menu (it uses `.first()`, so check which control it hits). Verify: the spec passes.

## 3. Verification

- [ ] 3.1 Run `scripts/run-tests.sh` with no flags; all suites (web, rust, contract, e2e) pass.
- [ ] 3.2 Manually check with `scripts/run-local.sh` on desktop and mobile widths: header shows no mark-all icon; detail pane shows "Mark watched" → "Watched" after clicking; title keeps full width on mobile.
