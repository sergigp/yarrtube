import type { Preferences } from '@/api/types'

/** The color theme the UI is drawn in. */
export type Theme = Preferences['theme']

/**
 * Where the last drawn theme is cached, so `index.html` can draw it before
 * first paint; the saved preference stays the source of truth.
 */
export const THEME_KEY = 'yarrtube.theme'
