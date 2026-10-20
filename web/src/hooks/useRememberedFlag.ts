import { useState } from 'react'

/** Storage unreadable or unwritable: starts at `initial`, lasts until reload. */
export function useRememberedFlag(key: string, initial: boolean): readonly [boolean, () => void] {
  const [flag, setFlag] = useState(() => {
    const stored = window.localStorage.getItem(key)
    return stored === null ? initial : stored === 'true'
  })

  const toggle = () => {
    const next = !flag
    setFlag(next)
    window.localStorage.setItem(key, String(next))
  }

  return [flag, toggle] as const
}
