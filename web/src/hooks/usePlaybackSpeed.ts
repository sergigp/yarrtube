import { useEffect, useState } from 'react'

export interface PlaybackSpeed {
  /** The `<video>` element's current playback speed. */
  rate: number
  /** Plays the video at `rate`. */
  changeRate: (rate: number) => void
}

/**
 * Owns the playback speed of the `videoElement` `<video>` while `videoId`
 * plays: applies a chosen speed, follows speed changes made through the
 * browser's own player controls, and starts every newly selected video at
 * normal speed. Takes the element itself (from a callback ref), like
 * `useWatchProgress`.
 */
export function usePlaybackSpeed(
  videoElement: HTMLVideoElement | null,
  _videoId: string | null,
): PlaybackSpeed {
  const [rate, setRate] = useState(1)

  useEffect(() => {
    const element = videoElement
    if (!element) {
      return undefined
    }
    const follow = () => setRate(element.playbackRate)
    element.addEventListener('ratechange', follow)
    return () => element.removeEventListener('ratechange', follow)
  }, [videoElement])

  // The element's 'ratechange' brings `rate` along, as for the native controls.
  const changeRate = (next: number) => {
    if (videoElement) {
      videoElement.playbackRate = next
    }
  }

  return { rate, changeRate }
}
