const HOUR_MS = 60 * 60 * 1000
const DAY_MS = 24 * HOUR_MS

/** Largest unit first: a year is 365 days and a month 30. */
const AGE_UNITS: [string, number][] = [
  ['y', 365 * DAY_MS],
  ['mo', 30 * DAY_MS],
  ['w', 7 * DAY_MS],
  ['d', DAY_MS],
  ['h', HOUR_MS],
]

/**
 * How long ago `published` (an ISO timestamp) was, in its largest whole unit
 * ("3h ago", "2d ago", "5w ago", "4mo ago", "2y ago"): "just now" under an
 * hour or ahead of now, `null` when the date is unknown or unreadable.
 */
export function formatAge(published: string | null): string | null {
  const time = published ? new Date(published).getTime() : Number.NaN
  if (Number.isNaN(time)) {
    return null
  }
  const elapsed = Date.now() - time
  const unit = AGE_UNITS.find(([, unitMs]) => elapsed >= unitMs)
  return unit ? `${Math.floor(elapsed / unit[1])}${unit[0]} ago` : 'just now'
}
