import { useEffect, useLayoutEffect, useRef } from 'react'
import { PHONE_LANDSCAPE_QUERY, useIsPhoneLandscape } from './useIsPhoneLandscape'

/**
 * Whether the player takes over the screen: a phone in landscape and
 * `hasVideo`. While it does, the page can't scroll; once it ends the page
 * scrolls back to where it was in portrait.
 */
export function useLandscapeTakeover(hasVideo: boolean): boolean {
  const takeover = useIsPhoneLandscape() && hasVideo
  // In landscape the page takes the desktop layout, where the document doesn't
  // scroll: its position is lost then, so only portrait positions count.
  const portraitScrollY = useRef(0)

  useEffect(() => {
    const record = () => {
      if (!window.matchMedia(PHONE_LANDSCAPE_QUERY).matches) {
        portraitScrollY.current = window.scrollY
      }
    }
    record()
    window.addEventListener('scroll', record, { passive: true })
    return () => window.removeEventListener('scroll', record)
  }, [])

  useLayoutEffect(() => {
    if (!takeover) {
      return
    }
    const root = document.documentElement
    const overflow = root.style.overflow
    root.style.overflow = 'hidden'
    return () => {
      root.style.overflow = overflow
      // Back to portrait, not leaving the page while still in landscape.
      if (!window.matchMedia(PHONE_LANDSCAPE_QUERY).matches) {
        window.scrollTo(0, portraitScrollY.current)
      }
    }
  }, [takeover])

  return takeover
}
