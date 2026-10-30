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
}

export interface SectionView<T> {
  /** True when only the heading shows. */
  folded: boolean
  /** Folded, and the viewed entry belongs to this section. */
  headingActive: boolean
  /** Every row, or `[]` when folded; undefined while loading. */
  rows: T[] | undefined
}

/** What a section shows: only its heading when folded, otherwise every row. */
export function sectionView<T extends { id: string }>(
  items: T[] | undefined,
  { folded, activeId }: SectionViewOptions,
): SectionView<T> {
  if (folded) {
    const headingActive = items?.some((item) => item.id === activeId) ?? false
    return { folded: true, headingActive, rows: [] }
  }
  return { folded: false, headingActive: false, rows: items }
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

/**
 * A sidebar row's menu control: where the device can hover, hidden (keeping
 * its space) until the row (`group/row`) is hovered or focused, or its menu
 * is open; always shown on the active row and on touch devices.
 */
export function rowMenuClassName(active: boolean): string {
  return active
    ? ''
    : '[@media(hover:hover)]:opacity-0 [@media(hover:hover)]:group-hover/row:opacity-100 [@media(hover:hover)]:group-focus-within/row:opacity-100 [@media(hover:hover)]:data-[state=open]:opacity-100'
}
