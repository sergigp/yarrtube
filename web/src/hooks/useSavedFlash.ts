import { useEffect, useState } from 'react'

const SAVED_SHOWN_MS = 2000

/**
 * Whether "Saved" shows: from each `flash()` for `SAVED_SHOWN_MS`, restarting
 * on the next, until `clear()`.
 */
export function useSavedFlash(): { shown: boolean; flash: () => void; clear: () => void } {
  const [savedAt, setSavedAt] = useState<number | null>(null)
  useEffect(() => {
    if (savedAt === null) {
      return undefined
    }
    const timer = setTimeout(() => setSavedAt(null), SAVED_SHOWN_MS)
    return () => clearTimeout(timer)
  }, [savedAt])
  return {
    shown: savedAt !== null,
    flash: () => setSavedAt(Date.now()),
    clear: () => setSavedAt(null),
  }
}
