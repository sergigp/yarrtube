# Tasks

Frontend-only change: the TDD cycles run in the Vitest suite (`web/`), with `npm run test` as the full-suite check in place of `cargo test`. Updating an existing dialog test whose asserted behavior a cycle deliberately changes counts as part of that cycle's green step.

## 1. Walking skeleton

- [x] 1.1 Create every new file from design.md with trivial bodies, wired but inert: `web/src/lib/saveLocations.ts` (`deriveSaveCandidates` returns `[{ path: defaultParent, count: 0 }]`, `readRememberedParent` returns `null`, `writeRememberedParent` no-ops), `web/src/hooks/useSaveLocation.ts` (returns a `SaveLocation` built on the existing `LocationField` state logic's defaults, hardcoded candidates), `web/src/components/SaveToField.tsx`, `FolderBrowser.tsx`, `FolderNameField.tsx` (render minimal markup from their props). Do not touch `LocationField`, `DestinationNotice` or the dialogs yet. Done when: `npm run check` passes with every existing test green.

## 2. Behaviour (TDD)

- [x] 2.1 `shows the save-to list with the default parent selected on open` — AddPlaylistDialog renders `SaveToField` below the URL field; list visible before any input with `playlists/` selected. Verified by the new test passing and `npm run test` green.
- [ ] 2.2 `suggests parent folders of tracked playlists with their counts` — tracked playlists under `playlists/kids/*` yield a `playlists/kids` candidate showing its 2 items. Verified by the test.
- [ ] 2.3 `selects a suggested folder in one click and the notice follows` — clicking the candidate updates the destination notice to `<root>/playlists/kids/<slug>`. Verified by the test.
- [ ] 2.4 `orders suggestions by most recent use and caps them` — with 6 distinct parents, the default plus the 4 most recently used are shown. Verified by the test.
- [ ] 2.5 `preselects the remembered parent when still suggested` — seeded storage preselects `playlists/kids` on open. Verified by the test.
- [ ] 2.6 `falls back to the default parent when the remembered one is stale` — remembered parent with no tracked items falls back to `playlists/` without an error. Verified by the test.
- [ ] 2.7 `remembers the submitted parent for the next open` — successful submit into `playlists/kids` preselects it when the dialog reopens. Verified by the test.
- [ ] 2.8 `opens the browser from choose-another and keeps the chosen folder selected` — "Choose another folder…" reveals `FolderBrowser` on the current parent; after browsing to a subfolder and closing, the list shows it selected and the notice matches. Verified by the test.
- [ ] 2.9 `shows no change action in the notice` — info and destination-occupied error notices render without a "change" link (`DestinationNotice` loses `onChange`). Verified by the test and by removing the now-dead assertions from existing dialog tests.
- [ ] 2.10 `keeps the folder name and quality under advanced options` — Advanced options collapsed by default hold `FolderNameField` (auto-fill, manual-edit stickiness, `/` and empty rejection intact) and quality; the parent control is gone from Advanced. Verified by the test.
- [ ] 2.11 `suggests only parents of tracked channels in the channel dialog` — AddChannelDialog gets the same layout; playlist-only parents are absent from its list. Verified by the test.
- [ ] 2.12 `keeps a separate remembered parent per dialog` — a playlist submit leaves the channel dialog's preselection untouched. Verified by the test.

## 3. Infrastructure adapters (TDD)

`web/src/lib/saveLocations.ts` is the change's only adapter-like module (pure derivation + localStorage boundary):

- [ ] 3.1 `derives distinct parents with counts from item paths` — replaces the skeleton's hardcoded return. Verified by the test.
- [ ] 3.2 `pins the default parent first even with zero occupants` — verified by the test.
- [ ] 3.3 `orders by newest created_at, then count, then name, and applies the cap` — verified by the test.
- [ ] 3.4 `reads null and writes without throwing when storage is unavailable` — read/write wrapped against throwing storage. Verified by the test.

## 4. Verification

- [ ] 4.1 Delete `LocationField.tsx` and `LocationField.test.tsx` once nothing imports them; migrate any still-relevant browser assertions into `SaveToField.test.tsx`. Verified by `npm run check` green.
- [ ] 4.2 Design pass: widen both dialogs to `sm:max-w-lg`, open up vertical spacing so the form breathes, and ensure long "in use by …" rows truncate inside the dialog with no horizontal clipping. Verified manually via `scripts/run-local.sh` on both dialogs (open browser panel, long labels, narrow viewport).
- [ ] 4.3 `npm run check` (typecheck + lint + full Vitest suite) and `cargo build --release` (embeds `dist/`) both pass; manual end-to-end add of a playlist into a nested folder via `scripts/run-local.sh`.
