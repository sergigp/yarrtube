import type { ChannelListItem, LibraryItem } from '@/api/types'

// When the search field appears.
export const SEARCH_THRESHOLD = 15

/**
 * Channels with unwatched videos first, most unwatched first. The sort is
 * stable, so ties and the caught-up channels keep the API's name order.
 */
export function orderChannels(channels: ChannelListItem[]): ChannelListItem[] {
  return [...channels].sort((a, b) => b.unwatched_count - a.unwatched_count)
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
