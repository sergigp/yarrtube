import { useIsPhoneLandscape } from './useIsPhoneLandscape'

/**
 * Whether the player takes over the screen: a phone in landscape and
 * `hasVideo`. While it does, the page can't scroll; once it ends the page
 * scrolls back to where it was in portrait.
 */
export function useLandscapeTakeover(hasVideo: boolean): boolean {
  return useIsPhoneLandscape() && hasVideo
}
