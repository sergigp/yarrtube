## 1. Walking skeleton

- [x] 1.1 Create every type and signature from design.md › Types & Signatures, wired end-to-end: `useMediaQuery` (real: the `useSyncExternalStore` body moved from `useIsDesktop`, which now calls it), `useIsPhoneLandscape` + `PHONE_LANDSCAPE_QUERY`, `useAutoplay` (no-op body), `useLandscapeTakeover` (returns `false`), both called from `VideoPlayer` (`useAutoplay(videoElement, video?.id ?? null, filename !== null, autoplay)`, `useLandscapeTakeover(filename !== null)`); test doubles: `viewport.phoneLandscape` + `rotatePhone()` in `src/test/viewport.ts`, a query-aware `matchMedia` stub with change listeners and a resolving `HTMLMediaElement.prototype.play` stub in `src/test/setup.ts`. Done when `scripts/run-tests.sh --lint --web` passes with no new tests.

## 2. Behaviour (TDD)

- [x] 2.1 `useVideoSelection.test.tsx` › `lets a manual pick override the deep link, with autoplay`: a pick from the list autoplays (update the hook's `autoplay` doc comment).
- [x] 2.2 `VideoPlayer.test.tsx` › `starts playing a downloaded video selected with autoplay`: `useAutoplay` calls `play()`, the `autoPlay` attribute goes; in the same cycle switch `ChannelDetail.test.tsx` / `CategoryDetail.test.tsx` autoplay-attribute assertions to a `play()` spy.
- [x] 2.3 `does not start a video selected without autoplay`.
- [x] 2.4 `starts each newly picked video in the same video element`.
- [x] 2.5 `does not start the video again when it re-renders`.
- [ ] 2.6 `does not start a video that finishes downloading while selected`.
- [ ] 2.7 `stays paused without an error when the browser refuses to play`.
- [ ] 2.8 `takes over the screen with a downloaded video on a phone in landscape`: fixed full-viewport black root with safe-area padding and `data-landscape-takeover`, video `object-contain`, no `md:` sizing classes.
- [ ] 2.9 `hides the chapter rail while taking over`.
- [ ] 2.10 `does not take over without a downloaded video`.
- [ ] 2.11 `keeps the same video element when rotating in and out`.
- [ ] 2.12 `locks page scroll while taking over and scrolls back on return`: `useLandscapeTakeover` tracks portrait `scrollY`, locks `<html>` overflow, restores both.

## 3. End-to-end (TDD)

- [ ] 3.1 `mobileWatch.spec.js` › home card test asserts the video is playing (not paused, `currentTime` advancing) before `assertVideoPlays`.
- [ ] 3.2 `picking a video in the queue starts it playing @mobile`.
- [ ] 3.3 `rotating to landscape fills the screen with the video and back restores the page @mobile` (if WebKit emulation doesn't report `pointer: coarse`, note it in the summary and leave the check to Manual (human)).

## 4. Verification

- [ ] 4.1 Add an `## [Unreleased]` entry to `CHANGELOG.md` (picked videos autoplay; phone landscape fills the screen with the video).
- [ ] 4.2 `scripts/run-tests.sh --lint --web --rust` passes.
- [ ] 4.3 `scripts/run-tests.sh --e2e` passes (e2e specs changed).
- [ ] 4.4 Manual (agent): `scripts/run-local.sh --port <free port> --fresh-videos`, add a playlist via the HTTP API, wait for downloads; with the Playwright MCP browser at 390×664: open a home card → video not paused; pick another queue video → it plays, same element; scroll 300 px, resize to 844×390 with touch emulation → screenshot shows only the video on black; resize back → `scrollY` restored, still playing. Stop the server, delete `yarrtube.sqlite3` and the fresh videos dir.
- [ ] 4.5 Manual (human): on an iPhone (Safari and home-screen app) and an Android phone: card tap starts playing (or stays paused, no error, if refused); queue pick starts playing; rotating to landscape while playing and while paused fills the screen, clear of the notch, native controls work; rotating back restores the page and scroll, playback uninterrupted; no takeover on a pending video.
