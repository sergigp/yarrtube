import type { LibraryItem } from '@/api/types'

// When the search field appears.
export const SEARCH_THRESHOLD = 15

/** The unwatched count to display: 0 when the entry hides it. */
export function shownUnwatchedCount(item: LibraryItem): number {
  return item.hide_unwatched_count ? 0 : (item.unwatched_count ?? 0)
}

/**
 * Entries with a shown unwatched count first, most first. The sort is stable,
 * so ties and the caught-up entries keep the API's name order.
 */
export function orderByUnwatched<T extends LibraryItem>(items: T[]): T[] {
  return [...items].sort((a, b) => shownUnwatchedCount(b) - shownUnwatchedCount(a))
}

/** Case-insensitive name match; expects `text` already trimmed. */
export function matchesSearch(item: { name: string }, text: string): boolean {
  return item.name.toLowerCase().includes(text.toLowerCase())
}

export interface SectionViewOptions {
  searchText: string
  folded: boolean
  activeId: string | null
}

export interface SectionView {
  /** True when only the heading shows: folded and not searching. */
  folded: boolean
  /** Folded, and the viewed entry belongs to this section. */
  headingActive: boolean
  /** Every row, the search matches, or `[]` when folded; undefined while loading. */
  rows: LibraryItem[] | undefined
}

/**
 * What a section shows: every match while searching, only its heading when
 * folded, otherwise every row.
 */
export function sectionView(
  items: LibraryItem[] | undefined,
  { searchText, folded, activeId }: SectionViewOptions,
): SectionView {
  if (folded && !searchText) {
    const headingActive = items?.some((item) => item.id === activeId) ?? false
    return { folded: true, headingActive, rows: [] }
  }
  const rows = items && searchText ? items.filter((item) => matchesSearch(item, searchText)) : items
  return { folded: false, headingActive: false, rows }
}

/** Sum of the rows' unwatched counts (missing counts as 0). */
export function unwatchedTotal(items: LibraryItem[]): number {
  return items.reduce((total, item) => total + (item.unwatched_count ?? 0), 0)
}

/** The entry id right after `prefix` in `pathname` (e.g. `/channels/<id>`), if any. */
export function activeIdFrom(pathname: string, prefix: string): string | null {
  if (!pathname.startsWith(prefix)) {
    return null
  }
  const rest = pathname.slice(prefix.length).split('/')[0]
  return rest ? decodeURIComponent(rest) : null
}
