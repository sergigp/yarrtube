# Design

## Files

Web only.

- `web/src/lib/watchProgress.ts` (new): `watchProgress` (the in-progress rule: percent + seconds left, or `null`) and `formatTimeLeft`. Pure, so it can be unit tested.
- `web/src/components/ThumbnailOverlay.tsx` (new): the tick, progress bar and duration / time-left badge on a thumbnail. The grid and the list share it so they can't drift. It has a `size` for card vs row.
- `web/src/components/WatchProgressBar.tsx`: takes `percent`, thicker per `size`, and a `bg-black/50` track.
- `web/src/components/WatchedTick.tsx`: takes a `size`; it sits above the bar when one is shown.
- `web/src/components/VideoGrid.tsx`:
  - Uses `ThumbnailOverlay`, and drops the `showProgress` prop.
  - Watched card: thumbnail `opacity-60`, back to full on `group-hover`, and a muted title.
- `web/src/components/VideoListPane.tsx`:
  - Uses `ThumbnailOverlay`.
  - Watched row: dimmed thumbnail and a muted title. The active row keeps `text-primary`.
- `web/src/components/Home.tsx`: drops `showProgress`.

Smoke:
- `smoke-tests/tests/playlist.spec.js`: the home card shows the time left, and a new step checks the progress bar and time left in the playlist's video list.

## Types & Signatures

```ts
// lib/watchProgress.ts
export interface WatchProgress {
  /** 0..100, saved position over duration. */
  percent: number
  /** Duration minus saved position, >= 0. */
  secondsLeft: number
}

/** In progress = unwatched, position > 0, duration known and > 0; otherwise null. */
export function watchProgress(video: {
  watched: boolean
  position_seconds: number
  duration_seconds: number | null
}): WatchProgress | null

/** "<1m left", "8m left", "1h left", "1h 5m left" (minutes rounded down). */
export function formatTimeLeft(seconds: number): string

// components/ThumbnailOverlay.tsx
type ThumbnailSize = 'card' | 'row'

export function ThumbnailOverlay(props: {
  watched: boolean
  positionSeconds: number
  durationSeconds: number | null
  size: ThumbnailSize
}): JSX.Element

// components/WatchProgressBar.tsx
export function WatchProgressBar(props: { percent: number; size: ThumbnailSize }): JSX.Element

// components/WatchedTick.tsx
export function WatchedTick(props: { watched: boolean; size: ThumbnailSize; raised: boolean }): JSX.Element | null

// components/VideoGrid.tsx
export function VideoGrid(props: { videos: HomeVideo[] }): JSX.Element
```

`ThumbnailSize` is exported from `ThumbnailOverlay.tsx`.

## Call Stack

```
VideoGrid (card) | VideoListPane (row)
  Thumbnail          className += watched ? 'opacity-60 [group-hover:opacity-100 on cards]' : ''
  ThumbnailOverlay   { watched, positionSeconds, durationSeconds, size }
    watchProgress({ watched, position_seconds, duration_seconds }) -> progress | null
    WatchedTick      { watched, size, raised: progress != null }   // watched => progress is null, kept for safety
    WatchProgressBar { percent: progress.percent, size }           // only when progress
    badge text = progress ? formatTimeLeft(progress.secondsLeft)
                          : formatDuration(durationSeconds)        // raised above the bar when progress
  title              className += watched ? 'text-muted-foreground' (unless active row)
```

## Test Plan

Vitest, ordered pure logic first, then components. There are no infrastructure tests (no backend change).

1. Behaviour tests
   1. `watchProgress.test.ts`, `returns progress for an unwatched started video`: 120/600 → `{ percent: 20, secondsLeft: 480 }`.
   2. `watchProgress.test.ts`, `returns null for an unstarted, watched or durationless video`.
   3. `watchProgress.test.ts`, `clamps a position past the duration`: → `{ percent: 100, secondsLeft: 0 }`.
   4. `watchProgress.test.ts`, `formats time left in minutes`: 480 → `8m left`, 539 → `8m left`.
   5. `watchProgress.test.ts`, `formats time left under a minute`: 45 and 0 → `<1m left`.
   6. `watchProgress.test.ts`, `formats time left from one hour`: 3600 → `1h left`, 3900 → `1h 5m left`.
   7. `ChannelDetail.test.tsx`, `shows progress and time left on an in-progress row`: one `progressbar` with `aria-valuenow` 20, plus the text `8m left`. The unstarted row shows its duration.
   8. `PlaylistDetail.test.tsx`, `shows progress and time left on an in-progress row`.
   9. `CategoryDetail.test.tsx`, `shows progress and time left on an in-progress card`.
   10. `Home.test.tsx`, `shows progress on in-progress cards in every section`: an in-progress video under "Latest videos" gets a `progressbar` and `Nm left`. This replaces the continue-watching-only count assertion in `shows every section that has videos`.
   11. `Home.test.tsx`, `shows no progress for watched or durationless cards`: no `progressbar`, and the duration or no badge.

Dimming (opacity and the muted title) is pure styling with no accessible signal, so it's checked in the manual verification, not in Vitest.
