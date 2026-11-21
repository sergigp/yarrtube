import { useMediaQuery } from './useMediaQuery'

/** The desktop breakpoint, Tailwind's `md`. */
export const DESKTOP_QUERY = '(min-width: 48rem)'

/** Whether the viewport is at or above the desktop breakpoint, kept current. */
export function useIsDesktop(): boolean {
  return useMediaQuery(DESKTOP_QUERY)
}
