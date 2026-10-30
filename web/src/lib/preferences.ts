import type { Preferences } from '@/api/types'

export const DEFAULT_PREFERENCES: Preferences = {
  categories_enabled: true,
  home_show_continue_watching: true,
  home_show_categories: true,
  home_show_quick_watches: true,
  home_show_latest: true,
  theme: 'light',
}

/**
 * The preferences to show the UI with: none while loading, so preference-gated
 * UI stays hidden instead of flashing, and the defaults when loading failed.
 */
function effective(preferences: Preferences | undefined, failed: boolean): Preferences | null {
  return preferences ?? (failed ? DEFAULT_PREFERENCES : null)
}

/** Whether category UI shows. */
export function categoriesShown(preferences: Preferences | undefined, failed: boolean): boolean {
  return effective(preferences, failed)?.categories_enabled ?? false
}
