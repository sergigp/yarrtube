import { useState } from 'react'

/** Storage unreadable or unwritable: starts at `initial`, lasts until reload. */
export function useRememberedFlag(key: string, initial: boolean): readonly [boolean, () => void] {
  void key
  const [flag, setFlag] = useState(initial)
  return [flag, () => setFlag(!flag)] as const
}
