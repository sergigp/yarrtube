import { useEffect, useState } from 'react'

const EVENTS = ['timeupdate', 'seeked', 'loadedmetadata', 'emptied'] as const

/** The playback position of `element`, in seconds, kept current. */
export function useCurrentTime(element: HTMLVideoElement | null): number {
  const [time, setTime] = useState(0)

  useEffect(() => {
    if (!element) {
      return
    }
    const follow = () => setTime(element.currentTime)
    follow()
    for (const event of EVENTS) {
      element.addEventListener(event, follow)
    }
    return () => {
      for (const event of EVENTS) {
        element.removeEventListener(event, follow)
      }
    }
  }, [element])

  return time
}
