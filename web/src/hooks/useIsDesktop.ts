import { useSyncExternalStore } from 'react'

/** The desktop breakpoint, Tailwind's `md`. */
export const DESKTOP_QUERY = '(min-width: 48rem)'

/** Whether the viewport is at or above the desktop breakpoint, kept current. */
export function useIsDesktop(): boolean {
  return useSyncExternalStore(subscribe, () => window.matchMedia(DESKTOP_QUERY).matches)
}

function subscribe(onChange: () => void): () => void {
  const query = window.matchMedia(DESKTOP_QUERY)
  query.addEventListener('change', onChange)
  return () => query.removeEventListener('change', onChange)
}
