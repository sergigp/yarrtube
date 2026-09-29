// How many rows a collapsed section leads with, and when the search field
// appears.
export const UNREAD_CAP = 10
export const CAUGHT_UP_PREVIEW = 5
export const SEARCH_THRESHOLD = 15

/**
 * Channels with unwatched videos first, most unwatched first. The sort is
 * stable, so ties and the caught-up channels keep the API's name order.
 */
export function orderChannels(channels) {
  return [...channels].sort((a, b) => b.unwatched_count - a.unwatched_count)
}

/**
 * The first `leadCount` rows, plus the row with `activeId` when it falls
 * outside them, so the entry being viewed is never hidden.
 */
export function collapse(rows, leadCount, activeId) {
  const lead = rows.slice(0, leadCount)
  const active = rows.slice(leadCount).find((row) => row.id === activeId)
  const shown = active ? [...lead, active] : lead
  return { shown, hiddenCount: rows.length - shown.length }
}

/**
 * The unread channels, up to `UNREAD_CAP`, or `CAUGHT_UP_PREVIEW` channels
 * when none is unread. Expects channels ordered by `orderChannels`.
 */
export function channelLeadCount(orderedChannels) {
  const unread = orderedChannels.filter((channel) => channel.unwatched_count > 0).length
  return Math.min(unread, UNREAD_CAP) || CAUGHT_UP_PREVIEW
}

export function matchesSearch(item, text) {
  return item.name.toLowerCase().includes(text.trim().toLowerCase())
}
