# Design

Web only, no Rust: the Test Plan's "behaviour tests" are Vitest component
tests driving `VideoPlayer` (the shared entry point of the playlist,
channel and category players) with `renderWithProviders`/`mockApi`; pure
logic gets plain unit tests. No infrastructure adapters change.

## Files

- `web/src/lib/chapters.ts` — pure chapter logic (current chapter, progress), unit tested per the `src/lib/` convention.
- `web/src/lib/chapters.test.ts` — its unit tests.
- `web/src/hooks/useCurrentTime.ts` — follows a `<video>`'s `currentTime` (`timeupdate`, `seeked`, `loadedmetadata`), like `usePlaybackSpeed` follows `ratechange`.
- `web/src/components/ChapterRail.tsx` — the horizontal rail of chapter cards plus, on mobile, the "All chapters" control; owns auto-scroll.
- `web/src/components/ChapterListSheet.tsx` — the mobile full list, built on `ui/dialog` positioned as a bottom sheet.
- `web/src/components/VideoPlayer.tsx` — wraps video + rail in one sticky container (so they pin together on mobile), renders the rail, seeks/plays on pick, reduces desktop `max-h` when the rail shows.
- `web/src/components/VideoPlayer.test.tsx` — behaviour tests below.

## Types & Signatures

```ts
// lib/chapters.ts
import type { Chapter } from '@/api/types'

/** Index of the chapter containing `time` (start inclusive, end exclusive), or null. */
export function currentChapterIndex(chapters: Chapter[], time: number): number | null

/** 0..1 progress of `time` through `chapter`, clamped. */
export function chapterProgress(chapter: Chapter, time: number): number
```

```ts
// hooks/useCurrentTime.ts
export function useCurrentTime(element: HTMLVideoElement | null): number
```

```ts
// components/ChapterRail.tsx
interface ChapterRailProps {
  chapters: Chapter[]
  currentTime: number
  onPick: (chapter: Chapter) => void
}
export function ChapterRail(props: ChapterRailProps): JSX.Element

// components/ChapterListSheet.tsx
interface ChapterListSheetProps {
  chapters: Chapter[]
  currentIndex: number | null
  open: boolean
  onOpenChange: (open: boolean) => void
  onPick: (chapter: Chapter) => void
}
export function ChapterListSheet(props: ChapterListSheetProps): JSX.Element
```

Card: `<button aria-current={isCurrent ? 'true' : undefined} title={chapter.title}>`
with `formatDuration(start_seconds)` and the title (`line-clamp-2`). Start
times reuse `lib/formatDuration`.

## Call Stack

Render:
```
PlaylistDetail | ChannelDetail | CategoryPlayer
  -> VideoPlayer({ video, ... })
       currentTime = useCurrentTime(videoElement)
       showRail = filename != null && video.chapters.length > 0
       -> ChapterRail({ chapters: video.chapters, currentTime, onPick })
            index = currentChapterIndex(chapters, currentTime)
            progress = chapterProgress(chapters[index], currentTime)
            useEffect([index]) -> list.scrollTo({ left: <card centred>, behavior: 'smooth' })   (rail only; scrollIntoView would also scroll the page)
            !useIsDesktop() -> "All chapters" button
                 -> ChapterListSheet({ chapters, currentIndex: index, open, onOpenChange, onPick })
```

Pick (rail card or sheet row):
```
onPick(chapter)
  -> VideoPlayer: videoElement.currentTime = chapter.start_seconds; void videoElement.play()
  -> ChapterListSheet: onOpenChange(false)   (sheet only)
```

Auto-scroll fires only when the current index changes, so a user
scrolling the rail is not fought every `timeupdate`.

## Test Plan

Behaviour (`VideoPlayer.test.tsx`, `describe('chapter rail')`), simplest first:
1. `shows a card per chapter with its start time and title, in order` — "0:00 Intro", "1:15 Game 1", "17:17 Game 2" as buttons.
2. `shows hour-long start times as h:mm:ss` — 3725 s → "1:02:05".
3. `shows no rail for a video without chapters` — `chapters: []`, no chapter buttons.
4. `shows no rail for a video that is not downloaded` — `status: 'PENDING'` with chapters.
5. `marks the chapter containing the playback position as current` — set `currentTime` 1500, dispatch `timeupdate`; "Game 2" has `aria-current`.
6. `moves the current mark when playback enters the next chapter` — 1000 → 1100 s; mark moves Game 1 → Game 2.
7. `marks no chapter before the first chapter starts` — first chapter at 10 s, time 5 s.
8. `seeks to a chapter start and plays when its card is clicked` — paused at 30 s, click "Game 2" → `currentTime` 1037, `play` called (jsdom: spy on `HTMLMediaElement.prototype.play`).
9. `scrolls the current card into view when the current chapter changes` — the rail's list `scrollTo` (stubbed in `test/setup.ts`) called once on a chapter change, not within a chapter.
10. `offers no "All chapters" control on desktop` — `matchMedia` desktop.
11. `lists every chapter in a sheet from "All chapters" on mobile, marking the current one` — mobile `matchMedia`, open sheet, rows + `aria-current`.
12. `seeks, plays and closes the sheet when a chapter is picked from it` — mobile.
13. `shows the rail in the category player` — `CategoryPlayer` rendered with a `CategoryVideo` with chapters (smoke test of sharing).

Unit (`lib/chapters.test.ts`):
- `currentChapterIndex`: inside, at exact start (inclusive), at exact end (belongs to next), before first, after last, empty list.
- `chapterProgress`: midpoint 0.5, clamped below 0 and above 1.

Not unit-testable in jsdom (manual): pinned-together sticky on mobile, desktop height budget, smooth scroll feel, sheet appearance.
