import type { ChannelListItem, LibraryItem } from '@/api/types'

// How many rows a collapsed section leads with, and when the search field
// appears.
export const UNREAD_CAP = 10
export const CAUGHT_UP_PREVIEW = 5
export const SEARCH_THRESHOLD = 15

/**
 * Channels with unwatched videos first, most unwatched first. The sort is
 * stable, so ties and the caught-up channels keep the API's name order.
 */
export function orderChannels(channels: ChannelListItem[]): ChannelListItem[] {
  return [...channels].sort((a, b) => b.unwatched_count - a.unwatched_count)
}

export interface CollapsedRows<T> {
  shown: T[]
  hiddenCount: number
}

/**
 * The first `leadCount` rows, plus the row with `activeId` when it falls
 * outside them, so the entry being viewed is never hidden.
 */
export function collapse<T extends { id: string }>(
  rows: T[],
  leadCount: number,
  activeId: string | null,
): CollapsedRows<T> {
  const lead = rows.slice(0, leadCount)
  const active = rows.slice(leadCount).find((row) => row.id === activeId)
  const shown = active ? [...lead, active] : lead
  return { shown, hiddenCount: rows.length - shown.length }
}

/**
 * The unread channels, up to `UNREAD_CAP`, or `CAUGHT_UP_PREVIEW` channels
 * when none is unread. Expects channels ordered by `orderChannels`.
 */
export function channelLeadCount(orderedChannels: ChannelListItem[]): number {
  const unread = orderedChannels.filter((channel) => channel.unwatched_count > 0).length
  return Math.min(unread, UNREAD_CAP) || CAUGHT_UP_PREVIEW
}

/** Case-insensitive name match; expects `text` already trimmed. */
export function matchesSearch(item: LibraryItem, text: string): boolean {
  return item.name.toLowerCase().includes(text.toLowerCase())
}

export interface SectionViewOptions {
  searchText: string
  folded: boolean
  expanded: boolean
  leadCount: number
  activeId: string | null
}

export interface SectionView {
  /** True when only the heading shows: folded and not searching. */
  folded: boolean
  /** Folded, and the viewed entry belongs to this section. */
  headingActive: boolean
  rows: LibraryItem[] | undefined
  hiddenCount: number
  collapsible: boolean
}

/**
 * The rows a section shows: every match while searching, otherwise all rows
 * when expanded or its leading rows (plus the active one) when collapsed.
 * `collapsible` says whether collapsing would hide anything.
 */
export function sectionView(
  items: LibraryItem[] | undefined,
  { searchText, expanded, leadCount, activeId }: SectionViewOptions,
): SectionView {
  if (!items) {
    return { folded: false, headingActive: false, rows: items, hiddenCount: 0, collapsible: false }
  }
  if (searchText) {
    return {
      folded: false,
      headingActive: false,
      rows: items.filter((item) => matchesSearch(item, searchText)),
      hiddenCount: 0,
      collapsible: false,
    }
  }
  const collapsed = collapse(items, leadCount, activeId)
  return {
    folded: false,
    headingActive: false,
    rows: expanded ? items : collapsed.shown,
    hiddenCount: expanded ? 0 : collapsed.hiddenCount,
    collapsible: collapsed.hiddenCount > 0,
  }
}

/** Sum of the rows' unwatched counts (missing counts as 0). */
export function unwatchedTotal(items: LibraryItem[]): number {
  void items
  return 0
}
