import { useMediaQuery } from './useMediaQuery'

/**
 * A phone held sideways: touch, landscape, and too short to be a tablet. Not
 * the desktop breakpoint: a phone in landscape is wider than it.
 */
export const PHONE_LANDSCAPE_QUERY =
  '(orientation: landscape) and (pointer: coarse) and (max-height: 32rem)'

/** Whether the viewport is a phone held in landscape, kept current. */
export function useIsPhoneLandscape(): boolean {
  return useMediaQuery(PHONE_LANDSCAPE_QUERY)
}
