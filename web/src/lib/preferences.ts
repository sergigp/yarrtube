import type { Preferences } from '@/api/types'

export const DEFAULT_PREFERENCES: Preferences = {
  categories_enabled: true,
  home_show_continue_watching: true,
  home_show_categories: true,
  home_show_category_colors: true,
  home_show_quick_watches: true,
  home_show_latest: true,
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

/** Whether home cards are tinted with their categories' colors. */
export function homeTintShown(preferences: Preferences | undefined, failed: boolean): boolean {
  const shown = effective(preferences, failed)
  return !!shown && shown.categories_enabled && shown.home_show_category_colors
}
