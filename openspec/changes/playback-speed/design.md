## Files

- `web/src/lib/playbackSpeed.ts` — offered speeds + label formatting; pure, so unit tested in `lib/`.
- `web/src/lib/playbackSpeed.test.ts` — unit tests for the above.
- `web/src/hooks/usePlaybackSpeed.ts` — owns the speed for the `<video>` element: applies it, follows native `ratechange`, resets to 1x per video. Mirrors `useWatchProgress` (takes the element from the callback ref).
- `web/src/hooks/usePlaybackSpeed.test.tsx` — hook tests against a real jsdom `<video>`.
- `web/src/components/PlaybackSpeedMenu.tsx` — presentational `1x ▾` dropdown (radio items), styled like `VideoActionsMenu`'s trigger.
- `web/src/components/PlaybackSpeedMenu.test.tsx` — component tests.
- `web/src/components/VideoDetail.tsx` — renders `PlaybackSpeedMenu` in the title row before `VideoActionsMenu`; disables it unless downloaded.
- `web/src/components/VideoDetail.test.tsx` — disabled-when-not-downloaded test.
- `web/src/components/PlaylistDetail.tsx` / `ChannelDetail.tsx` — call `usePlaybackSpeed(videoElement, selectedVideo?.id ?? null)` next to `useWatchProgress`, pass rate + setter to `VideoDetail`.
- `web/src/components/PlaylistDetail.test.tsx` — end-to-end: chosen speed reaches the `<video>`, resets on selecting another video.

## Types & Signatures

```ts
// lib/playbackSpeed.ts
export const PLAYBACK_SPEEDS: readonly number[] // [1, 1.1, 1.25, 1.5, 2]
export function formatPlaybackSpeed(rate: number): string // 1 -> "1x", 1.75 -> "1.75x"

// hooks/usePlaybackSpeed.ts
export interface PlaybackSpeed {
  rate: number
  changeRate: (rate: number) => void
}
export function usePlaybackSpeed(
  videoElement: HTMLVideoElement | null,
  videoId: string | null,
): PlaybackSpeed

// components/PlaybackSpeedMenu.tsx
interface PlaybackSpeedMenuProps {
  rate: number
  onRateChange: (rate: number) => void
  disabled: boolean
  className?: string
}
export function PlaybackSpeedMenu(props: PlaybackSpeedMenuProps): JSX.Element

// components/VideoDetail.tsx (changed)
interface VideoDetailProps {
  basePath: string
  video: Video
  channel?: ChannelListItem
  playbackRate: number
  onPlaybackRateChange: (rate: number) => void
}
```

Only `playbackRate` is touched, never `defaultPlaybackRate`, so the browser's own reset on `src` change agrees with ours.

## Call Stack

Choosing a speed:
```
PlaybackSpeedMenu (radio onValueChange "1.5")
  -> onRateChange(1.5)                       [VideoDetail.onPlaybackRateChange]
    -> changeRate(1.5)                       [usePlaybackSpeed in Playlist/ChannelDetail]
      -> videoElement.playbackRate = 1.5
      -> setRate(1.5)
```

Selecting another video:
```
Playlist/ChannelDetail re-renders with new selectedVideo.id
  -> usePlaybackSpeed effect [videoElement, videoId]
    -> videoElement.playbackRate = 1
    -> setRate(1)
```

Native player menu changes speed:
```
<video> 'ratechange' event
  -> usePlaybackSpeed listener -> setRate(videoElement.playbackRate)   // e.g. 1.75
    -> PlaybackSpeedMenu label "1.75x", no radio item checked
```

## Test Plan

Vitest, existing `it('...')` naming.

1. `lib/playbackSpeed.test.ts`
   1. `it('offers 1x, 1.1x, 1.25x, 1.5x and 2x')` — `PLAYBACK_SPEEDS` equals `[1, 1.1, 1.25, 1.5, 2]`.
   2. `it('formats a speed as its multiplier')` — `1 -> "1x"`, `1.1 -> "1.1x"`, `1.75 -> "1.75x"`.
2. `hooks/usePlaybackSpeed.test.tsx` (real `<video>` element)
   1. `it('starts at normal speed')` — `rate` is 1.
   2. `it('applies a chosen speed to the video element')` — after `changeRate(1.5)`, element `playbackRate` and `rate` are 1.5.
   3. `it('follows speed changes made by the native controls')` — set element `playbackRate = 1.75` + dispatch `ratechange` → `rate` 1.75.
   4. `it('resets to normal speed when another video is selected')` — `changeRate(2)`, rerender with new `videoId` → element `playbackRate` and `rate` are 1.
3. `components/PlaybackSpeedMenu.test.tsx`
   1. `it('shows the current speed and marks it in the menu')` — trigger reads "1.25x"; "1.25x" item checked.
   2. `it('reports the chosen speed')` — choosing "1.5x" calls `onRateChange(1.5)`.
   3. `it('marks no speed when the current one is not offered')` — rate 1.75: trigger "1.75x", no item checked.
4. `components/VideoDetail.test.tsx`
   1. `it('disables the speed control until the video is downloaded')` — `DOWNLOADING` video → speed trigger disabled.
5. `components/PlaylistDetail.test.tsx`
   1. `it('plays the selected video at the chosen speed and resets it for the next video')` — choose "2x" → `<video>` `playbackRate` 2; select another row → 1 and trigger "1x".
