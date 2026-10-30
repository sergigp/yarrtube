import { useEffect, useRef, useState } from 'react'

/** Smaller movements, such as iOS rubber-banding, don't change the direction. */
const MIN_DELTA_PX = 8

/**
 * The direction the document last scrolled: `up` at its top (overscroll
 * included) and from the first upward movement, `down` while scrolling down.
 */
export function useScrollDirection(): 'up' | 'down' {
  const [direction, setDirection] = useState<'up' | 'down'>('up')
  const lastY = useRef(0)

  useEffect(() => {
    lastY.current = Math.max(window.scrollY, 0)
    const onScroll = () => {
      const y = window.scrollY
      if (y <= 0) {
        lastY.current = 0
        setDirection('up')
        return
      }
      const delta = y - lastY.current
      if (Math.abs(delta) < MIN_DELTA_PX) {
        return
      }
      lastY.current = y
      setDirection(delta > 0 ? 'down' : 'up')
    }
    window.addEventListener('scroll', onScroll, { passive: true })
    return () => window.removeEventListener('scroll', onScroll)
  }, [])

  return direction
}
