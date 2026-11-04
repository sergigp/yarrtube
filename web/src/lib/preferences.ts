import type { Preferences } from '@/api/types'

/** The preferences the UI is drawn with, as opposed to AI settings. */
type DisplayPreferences = Pick<
  Preferences,
  | 'categories_enabled'
  | 'home_show_continue_watching'
  | 'home_show_categories'
  | 'home_show_quick_watches'
  | 'home_show_latest'
  | 'theme'
>

export const DEFAULT_PREFERENCES: DisplayPreferences = {
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
function effective(
  preferences: Preferences | undefined,
  failed: boolean,
): DisplayPreferences | null {
  return preferences ?? (failed ? DEFAULT_PREFERENCES : null)
}

/** Whether category UI shows. */
export function categoriesShown(preferences: Preferences | undefined, failed: boolean): boolean {
  return effective(preferences, failed)?.categories_enabled ?? false
}

/** Whether AI is on. */
export function aiEnabled(preferences: Preferences | undefined): boolean {
  return preferences?.ai_enabled ?? false
}

/** Whether sources can have their videos summarized: AI and summaries on. */
export function summariesAvailable(preferences: Preferences | undefined): boolean {
  return aiEnabled(preferences) && (preferences?.summaries_enabled ?? false)
}
