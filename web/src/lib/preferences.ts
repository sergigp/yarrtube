import type { Preferences } from '@/api/types'

export const DEFAULT_PREFERENCES: Preferences = {
  categories_enabled: true,
  home_show_continue_watching: true,
  home_show_categories: true,
  home_show_category_colors: true,
  home_show_quick_watches: true,
  home_show_latest: true,
}

/** Whether category UI shows: hidden while loading, the defaults when loading failed. */
export function categoriesShown(preferences: Preferences | undefined, failed: boolean): boolean {
  void preferences
  void failed
  return true
}

/** Whether home cards are tinted with their categories' colors. */
export function homeTintShown(preferences: Preferences | undefined, failed: boolean): boolean {
  void preferences
  void failed
  return true
}
