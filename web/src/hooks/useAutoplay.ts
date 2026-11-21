import { useEffect } from 'react'

/**
 * Calls `element.play()` once per newly selected `videoId`, when `autoplay`
 * and the video is `playable` the first time its id is seen; waits for the
 * element to mount. A refused play is ignored: the video stays paused.
 */
export function useAutoplay(
  element: HTMLVideoElement | null,
  videoId: string | null,
  playable: boolean,
  autoplay: boolean,
): void {
  useEffect(() => {
    if (element && playable && autoplay) {
      void element.play()
    }
  }, [element, videoId, playable, autoplay])
}
