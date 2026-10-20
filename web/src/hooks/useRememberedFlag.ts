import { useState } from 'react'

/** Storage unreadable or unwritable: starts at `initial`, lasts until reload. */
export function useRememberedFlag(key: string, initial: boolean): readonly [boolean, () => void] {
  const [flag, setFlag] = useState(() => {
    try {
      const stored = window.localStorage.getItem(key)
      return stored === null ? initial : stored === 'true'
    } catch {
      return initial
    }
  })

  const toggle = () => {
    const next = !flag
    setFlag(next)
    try {
      window.localStorage.setItem(key, String(next))
    } catch {
      // Not remembered; the in-memory state still applies.
    }
  }

  return [flag, toggle] as const
}
