import { useState } from 'react'

/**
 * One of `choices`, remembered in localStorage under `key`, e.g. a
 * playlist's sort. What's stored is checked against `choices` on every
 * render, so a value that isn't a choice yet (a source whose videos are
 * still loading) applies once it is, and one that is no longer a choice
 * falls back to `initial`. Storage that can't be read or written falls back
 * to `initial` too, with changes lasting until reload. Following another
 * `key` (another playlist on the same view) reads that key's value.
 */
export function useRememberedChoice<T extends string>(
  key: string,
  choices: readonly T[],
  initial: T,
): readonly [T, (value: T) => void] {
  const [state, setState] = useState(() => ({ key, stored: read(key) }))
  let current = state
  if (state.key !== key) {
    current = { key, stored: read(key) }
    setState(current)
  }

  const choose = (value: T) => {
    setState({ key, stored: value })
    try {
      window.localStorage.setItem(key, value)
    } catch {
      // Not remembered; the in-memory choice still applies.
    }
  }

  const value = choices.find((choice) => choice === current.stored) ?? initial
  return [value, choose] as const
}

function read(key: string): string | null {
  try {
    return window.localStorage.getItem(key)
  } catch {
    return null
  }
}
