# Tasks

## 1. Walking skeleton

- [x] 1.1 Create `web/src/lib/markWatchedConfirmation.ts` with `MarkWatchedConfirmation` and `markWatchedConfirmation()` returning `null`; add `pendingLabel?` to `ConfirmDialog` (default "Deleting…", not yet used); add the unused `confirmingMarkWatched` state + `ConfirmDialog` to `DetailHeader` and `pendingMarkWatched` state + `ConfirmDialog` to `SidebarSection`, both closed; fix the stale playlist comment in `web/src/api/types.ts`. Done when `npm run check` passes with no behaviour change.

## 2. Behaviour (TDD)

- [ ] 2.1 `markWatchedConfirmation` returns null for 0 and 1 unwatched — verify the unit test passes.
- [ ] 2.2 `markWatchedConfirmation` for 23 unwatched in "Foo" returns `Mark 23 videos in "Foo" watched?` and the can't-undo description — verify the unit test passes.
- [ ] 2.3 `ConfirmDialog` shows `pendingLabel` on the confirm button while `onConfirm` is pending — verify the test passes.
- [ ] 2.4 `DetailHeader` with 23 unwatched: "Mark all watched" opens a red confirmation ("Mark watched" / "Marking…") without marking; confirming calls `onMarkWatched` and closes — verify the test passes.
- [ ] 2.5 `DetailHeader`: cancelling never calls `onMarkWatched` — verify the test passes.
- [ ] 2.6 `DetailHeader` with 1 unwatched marks without a dialog — verify the test passes.
- [ ] 2.7 `DetailHeader`: a failure after confirming keeps the dialog open with the error — verify the test passes.
- [ ] 2.8 `CategoryDetail` with 3 unwatched asks for confirmation before `POST .../watched` is sent — verify the test passes.
- [ ] 2.9 `Sidebar` channel row with 2+ unwatched opens the confirmation; confirming sends `POST /api/channels/:id/watched` (update the existing mark-watched tests in `Sidebar.test.tsx` to confirm) — verify the tests pass.
- [ ] 2.10 `Sidebar` playlist row with 1 unwatched marks without a dialog — verify the test passes.
- [ ] 2.11 `Sidebar` playlist with hidden count and 4 unwatched still asks, saying 4 videos — verify the test passes.

## 3. Infrastructure adapters (TDD)

- [ ] 3.1 None: no adapter changes (frontend only). Mark done.

## 4. Verification

- [ ] 4.1 `npm run check` in `web/`, plus `cargo test --locked`, `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings` and end-to-end tests via `scripts/run-tests.sh` all pass (update any e2e test that marks a source watched to confirm the dialog).
- [ ] 4.2 Manual (agent): start `scripts/run-local.sh --port <free port> --fresh-videos`, add a channel with 2+ downloaded videos via the HTTP API, then with the Playwright MCP browser: from the channel header and from the sidebar row menu, "Mark all watched" shows the confirmation with the right count; Cancel sends no `POST .../watched` request; confirming sends it and the badge disappears; repeat once from a category view header. Screenshot the dialog. Stop the server and delete `yarrtube.sqlite3` and the fresh videos dir.
- [ ] 4.3 Manual (human): check on a phone that the confirmation opens from the header's "⋮" menu and reads well at narrow width.

## Workflow follow-up

- Archive the change after review (`/opsx:archive`).
