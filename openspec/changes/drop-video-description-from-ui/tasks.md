# Tasks

## 1. Walking skeleton

- [x] 1.1 Nothing to create, this change only removes code: confirm the baseline with `scripts/run-tests.sh --only web --only rust` passing.

## 2. Behaviour (TDD)

- [x] 2.1 Remove `description` from `CategoryVideoResponse` and update its whole-value listing tests; `cargo test --locked` passes.
- [x] 2.2 Remove `description` from `VideoResponse` and update its whole-value listing tests; `cargo test --locked` passes.
- [x] 2.3 VideoDetail `shows the storage location with no More control`: remove the toggle and the `Description` component, then `description` from `web/src/api/types.ts` and `aVideo`, deleting the tests they made obsolete; test passes.
- [x] 2.4 VideoDetail `shows the status of a video that is not downloaded`: test passes.
- [x] 2.5 Replace the "start collapsed" test in `e2e-tests/tests/mobileWatch.spec.js` with one asserting the storage location is visible and no "More" button exists; `scripts/run-tests.sh --only e2e` passes.

## 3. Infrastructure adapters (TDD)

No adapter changes.

## 4. Verification

- [x] 4.1 `scripts/run-tests.sh --only web --only rust` passes.
- [x] 4.2 Manual (agent): start `scripts/run-local.sh --port <free port> --fresh-videos`, seed a playlist through the HTTP API, and with the Playwright MCP browser check at desktop and mobile widths that the pane shows no description and no More/Less control, that the storage location is visible, and that the listing response has no `description`. Then stop the server and delete `yarrtube.sqlite3` and the fresh videos dir.
- [ ] 4.3 Manual (human): check in Plex that a newly synced video's overview still shows its description.
