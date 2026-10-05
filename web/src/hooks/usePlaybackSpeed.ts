import { useEffect, useState } from 'react'

export interface PlaybackSpeed {
  /** The selected video's playback speed; 1 until it plays at another. */
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
  // The speed last seen on the element, and which video it was playing. Any
  // other video or element reads as normal speed, even while no element
  // reports it: a new element starts at normal speed without a 'ratechange'.
  const [followed, setFollowed] = useState<FollowedSpeed | null>(null)
  const rate =
    followed?.element === videoElement && followed.videoId === videoId ? followed.rate : 1

  useEffect(() => {
    const element = videoElement
    if (!element) {
      return undefined
    }
    const follow = () => setFollowed({ element, videoId, rate: element.playbackRate })
    element.addEventListener('ratechange', follow)
    return () => element.removeEventListener('ratechange', follow)
  }, [videoElement, videoId])

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

interface FollowedSpeed {
  element: HTMLVideoElement
  videoId: string | null
  rate: number
}
