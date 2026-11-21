/**
 * Calls `element.play()` once per newly selected `videoId`, when `autoplay`
 * and the video is `playable` the first time its id is seen; waits for the
 * element to mount. A refused play is ignored: the video stays paused.
 */
export function useAutoplay(
  _element: HTMLVideoElement | null,
  _videoId: string | null,
  _playable: boolean,
  _autoplay: boolean,
): void {}
