# Tasks

## 1. Walking skeleton

- [x] 1.1 Create every file and signature from design.md (`lib/chapters.ts`, `hooks/useCurrentTime.ts`, `ChapterRail.tsx`, `ChapterListSheet.tsx`) with trivial bodies (`null`, `0`, an empty rail container), and wire `VideoPlayer` to wrap video + rail in one sticky container rendering `ChapterRail` only when downloaded with chapters. Done when `npm run check` passes with no new tests and the player looks unchanged for videos without chapters.

## 2. Behaviour (TDD)

Each task is one red-green-refactor cycle; verified by the named test passing and `npm run test` green.

- [x] 2.1 `shows a card per chapter with its start time and title, in order`
- [x] 2.2 `shows hour-long start times as h:mm:ss`
- [x] 2.3 `shows no rail for a video without chapters`
- [x] 2.4 `shows no rail for a video that is not downloaded`
- [x] 2.5 `marks the chapter containing the playback position as current` (drives `useCurrentTime` and `currentChapterIndex`)
- [x] 2.6 `moves the current mark when playback enters the next chapter`
- [x] 2.7 `marks no chapter before the first chapter starts`
- [x] 2.8 `seeks to a chapter start and plays when its card is clicked`
- [x] 2.9 `scrolls the current card into view when the current chapter changes` (the rail scrolls its own list; `scrollTo` stub added to `test/setup.ts`)
- [x] 2.10 `offers no "All chapters" control on desktop`
- [x] 2.11 `lists every chapter in a sheet from "All chapters" on mobile, marking the current one`
- [x] 2.12 `seeks, plays and closes the sheet when a chapter is picked from it`
- [x] 2.13 `shows the rail in the category player`
- [x] 2.14 Unit: `currentChapterIndex` edge cases from the Test Plan (exact start, exact end, after last, empty list) in `lib/chapters.test.ts`
- [x] 2.15 Unit: `chapterProgress` midpoint and clamping, driving the current card's progress fill

## 3. Infrastructure adapters (TDD)

None: no adapter changes.

## 4. Verification

- [x] 4.1 `cd web && npm run check` passes (typecheck, lint, tests).
- [x] 4.2 `scripts/run-tests.sh` with no flags passes (all suites).
- [x] 4.3 Manual (agent): start `scripts/run-local.sh --port <free port> --fresh-videos`, add via the HTTP API a playlist containing a short video with YouTube chapters, wait for the download and chapters, then with the Playwright MCP browser: desktop width — rail shows the cards in order, video + rail stay within 70vh when the video is height-capped (short viewport) and the video keeps its size when width-limited, clicking a card sets `currentTime` to its start and plays, current card has `aria-current`; mobile width (390 px) — after scrolling, player and rail are both pinned under the header (screenshot), "All chapters" opens the sheet, picking a row seeks and closes it. Stop the server and delete `yarrtube.sqlite3` and the fresh videos dir.
- [ ] 4.4 Manual (human): on a real phone, swiping the rail and the sheet feel right, the pinned player + rail doesn't take too much screen; on desktop, the auto-scroll is smooth and doesn't fight manual scrolling of the rail; overall visual judgement in light and dark themes.

## Workflow follow-up

- Archive the change after testing the iteration and folding feedback in.
