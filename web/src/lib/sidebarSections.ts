import { matchesSearch } from './search'
import type { LibraryItem } from '@/api/types'

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

export interface SectionViewOptions {
  folded: boolean
  activeId: string | null
  /** Trimmed filter text; while non-blank, the section shows every match, unfolded. */
  filter?: string
  /** Rows shown before "Show N more"; absent shows every row. */
  limit?: number
  /** "Show N more" was clicked: every row shows. */
  expanded?: boolean
}

export interface SectionView<T> {
  /** True when only the heading shows. */
  folded: boolean
  /** Folded, and the viewed entry belongs to this section. */
  headingActive: boolean
  /** The rows to show, `[]` when folded; undefined while loading. */
  rows: T[] | undefined
  /** Rows left out past the limit, offered as "Show N more". */
  hiddenCount: number
}

/**
 * What a section shows: only its heading when folded; every match while
 * filtering; otherwise its first `limit` rows (plus the viewed entry, if it
 * comes later) until expanded.
 */
export function sectionView<T extends { id: string; name: string }>(
  items: T[] | undefined,
  { folded, activeId, filter = '', limit, expanded = false }: SectionViewOptions,
): SectionView<T> {
  if (filter) {
    return {
      folded: false,
      headingActive: false,
      rows: items?.filter((item) => matchesSearch(item, filter)),
      hiddenCount: 0,
    }
  }
  if (folded) {
    const headingActive = items?.some((item) => item.id === activeId) ?? false
    return { folded: true, headingActive, rows: [], hiddenCount: 0 }
  }
  if (!items || limit === undefined || expanded || items.length <= limit) {
    return { folded: false, headingActive: false, rows: items, hiddenCount: 0 }
  }
  const rows = items.filter((item, index) => index < limit || item.id === activeId)
  return { folded: false, headingActive: false, rows, hiddenCount: items.length - rows.length }
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

// Where the device can hover, a row's unwatched count and its menu control
// share the trailing slot: the count by default, the menu while the row
// (`group/row`) is hovered, keyboard focused or has its menu open. Touch
// devices can't hover, so they keep both side by side.

/** A sidebar row's menu control, laid over its count slot where the device can hover. */
export function rowMenuClassName(): string {
  return '[@media(hover:none)]:mr-1 [@media(hover:hover)]:absolute [@media(hover:hover)]:inset-y-0 [@media(hover:hover)]:right-0 [@media(hover:hover)]:my-auto [@media(hover:hover)]:opacity-0 [@media(hover:hover)]:group-hover/row:opacity-100 [@media(hover:hover)]:group-has-[:focus-visible]/row:opacity-100 [@media(hover:hover)]:data-[state=open]:opacity-100'
}

/**
 * A sidebar row's trailing count slot, wide enough for the menu control laid
 * over it, and hidden while that control shows.
 */
export function rowCountSlotClassName(): string {
  return 'ml-auto flex min-w-5 shrink-0 justify-end [@media(hover:hover)]:group-hover/row:opacity-0 [@media(hover:hover)]:group-has-[:focus-visible]/row:opacity-0 [@media(hover:hover)]:group-has-[[data-state=open]]/row:opacity-0'
}
