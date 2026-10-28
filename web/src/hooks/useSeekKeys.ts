import { useEffect, useRef, useState } from 'react'
import {
  SEEK_FEEDBACK_MS,
  SEEK_SECONDS,
  type SeekDirection,
  seekDirection,
  seekTime,
} from '@/lib/seekKeys'

export interface SeekFeedback {
  direction: SeekDirection
  /** Seconds sought by the presses in a row so far, e.g. 20 after two. */
  seconds: number
  /** Changes on every press, to restart the feedback's animation. */
  press: number
}

/**
 * Seeks `videoElement` 10 s back or forward on ← / → anywhere on the page
 * (outside text fields), replacing the browser's own shorter step, and
 * reports each seek for on-screen feedback until it fades. Presses in quick
 * succession towards the same direction add up, like YouTube's.
 */
export function useSeekKeys(videoElement: HTMLVideoElement | null): SeekFeedback | null {
  const [feedback, setFeedback] = useState<SeekFeedback | null>(null)
  const hideTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)

  useEffect(() => {
    const element = videoElement
    if (!element) {
      return undefined
    }
    const seek = (event: KeyboardEvent) => {
      const direction = seekDirection(event)
      if (!direction) {
        return
      }
      event.preventDefault()
      element.currentTime = seekTime(element.currentTime, element.duration, direction)
      setFeedback((previous) => ({
        direction,
        seconds: previous?.direction === direction ? previous.seconds + SEEK_SECONDS : SEEK_SECONDS,
        press: (previous?.press ?? 0) + 1,
      }))
      clearTimeout(hideTimer.current)
      hideTimer.current = setTimeout(() => setFeedback(null), SEEK_FEEDBACK_MS)
    }
    document.addEventListener('keydown', seek)
    return () => {
      document.removeEventListener('keydown', seek)
      clearTimeout(hideTimer.current)
      setFeedback(null)
    }
  }, [videoElement])

  return feedback
}
