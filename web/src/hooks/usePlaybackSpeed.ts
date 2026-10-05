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
  videoId: string | null,
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

  // Every newly selected video starts at normal speed.
  useEffect(() => {
    if (videoElement) {
      setSpeed(videoElement, 1)
    }
  }, [videoElement, videoId])

  // The element's 'ratechange' brings `rate` along, as for the native controls.
  const changeRate = (next: number) => {
    if (videoElement) {
      setSpeed(videoElement, next)
    }
  }

  return { rate, changeRate }
}

function setSpeed(element: HTMLVideoElement, rate: number): void {
  element.playbackRate = rate
}
