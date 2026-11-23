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
  // From the takeover until the portrait position is back, scroll events only
  // report that loss: the document clamped to the top, possibly dispatched
  // once the viewport is portrait again but before it is restored.
  const paused = useRef(false)
  const restoreFrame = useRef<number | null>(null)

  useEffect(() => {
    const record = () => {
      if (!paused.current && !window.matchMedia(PHONE_LANDSCAPE_QUERY).matches) {
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
    paused.current = true
    // Turned back to landscape before the last takeover's restore ran.
    if (restoreFrame.current !== null) {
      cancelAnimationFrame(restoreFrame.current)
      restoreFrame.current = null
    }
    const root = document.documentElement
    const overflow = root.style.overflow
    root.style.overflow = 'hidden'
    return () => {
      root.style.overflow = overflow
      // On the next frame, once every media query listener has re-rendered
      // the page for portrait: scrolling now could be clamped by a layout
      // still the landscape one. Before the frame is painted, so no flash.
      restoreFrame.current = requestAnimationFrame(() => {
        restoreFrame.current = null
        // Back to portrait, not leaving the page while still in landscape.
        if (!window.matchMedia(PHONE_LANDSCAPE_QUERY).matches) {
          window.scrollTo(0, portraitScrollY.current)
        }
        paused.current = false
      })
    }
  }, [takeover])

  return takeover
}
