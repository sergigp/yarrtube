import { useEffect, useRef } from 'react'

/**
 * Calls `element.play()` once per newly selected `videoId`, when `autoplay`
 * and the video is `playable` the first time its id is seen; waits for the
 * element to mount. A refused play is ignored: the video stays paused.
 *
 * Calling `play()` rather than setting `autoplay` on the element is what lets
 * a pick from the list start on iOS: the <video> is reused and the call runs
 * within the tap, which iOS requires for playback with sound.
 */
export function useAutoplay(
  element: HTMLVideoElement | null,
  videoId: string | null,
  playable: boolean,
  autoplay: boolean,
): void {
  // The selection last acted on; re-renders and list polls repeat it.
  const handled = useRef<string | null>(null)

  useEffect(() => {
    const selection = videoId === null ? null : `${videoId}:${autoplay}`
    if (selection === handled.current) {
      return
    }
    if (!playable) {
      // Selected before it downloaded: finishing later doesn't start it.
      handled.current = selection
      return
    }
    if (!element) {
      return
    }
    handled.current = selection
    if (autoplay) {
      void element.play()
    }
  }, [element, videoId, playable, autoplay])
}
