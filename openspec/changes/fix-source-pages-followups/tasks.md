# Tasks

## 1. Walking skeleton

- [x] 1.1 Add the `sort: VideoSort` prop to `CategoryPlayerProps` and pass `sort` from `CategoryDetail` (unused for now); verify `npm run typecheck` and the web suite pass.

## 2. Behaviour (TDD)

- [x] 2.1 "names the remembered order in the queue header" (CategoryDetail.test.tsx): queue header follows the remembered sort; verify the test fails red then passes.
- [x] 2.2 "keeps the loaded videos when a refresh fails" (PlaylistDetail.test.tsx): `PlaylistBrowser` shows the error only without data; verify red then green.
- [ ] 2.3 "keeps the loaded videos when a refresh fails" (CategoryDetail.test.tsx): same for `CategoryDetail`; verify red then green.
- [ ] 2.4 "counts only listed rows in the queue position with watched hidden": `VideoListPane` position and total from the shown rows; verify red then green.
- [ ] 2.5 "counts unwatched videos of hidden playlists in the header" (CategoryDetail.test.tsx): header count from the listed videos; verify red then green.
- [ ] 2.6 "confirms marking every listed unwatched video" (CategoryDetail.test.tsx): mark-watched confirmation uses the same count; verify red then green.

## 3. Infrastructure adapters (TDD)

None: no adapter changes.

## 4. Verification

- [ ] 4.1 Run `scripts/run-tests.sh --lint --web`; all pass.
- [ ] 4.2 Manual (agent): start `scripts/run-local.sh --port <free port> --fresh-videos`, open a category with "Oldest first" chosen and play a video, check via the Playwright MCP browser that the queue header reads "… · oldest first"; with "Hide watched" on, check the position counts only listed rows; stop the server and delete `yarrtube.sqlite3` and the fresh videos dir.

## Workflow follow-up

- Finish with `/openspec:finish`: sync specs, archive, push, open the PR against `main` saying it builds on #202 (rebase onto `main` first if #202 is merged).
