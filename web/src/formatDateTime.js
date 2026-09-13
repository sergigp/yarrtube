/**
 * Formats an ISO timestamp as an absolute date/time, rounded to the minute
 * (no seconds component).
 */
export function formatDateTime(isoString) {
  if (!isoString) {
    return '—'
  }

  const date = new Date(isoString)
  if (Number.isNaN(date.getTime())) {
    return isoString
  }

  return date.toLocaleString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })
}

const RELATIVE_UNITS = [
  ['y', 31536000],
  ['mo', 2592000],
  ['d', 86400],
  ['h', 3600],
  ['m', 60],
  ['s', 1],
]

/**
 * Formats an ISO timestamp as a short relative time ("3s ago", "in 2m").
 */
export function formatRelativeTime(isoString) {
  if (!isoString) {
    return '—'
  }

  const date = new Date(isoString)
  if (Number.isNaN(date.getTime())) {
    return isoString
  }

  const diffSeconds = Math.round((date.getTime() - Date.now()) / 1000)
  const absSeconds = Math.abs(diffSeconds)

  if (absSeconds < 5) {
    return 'just now'
  }

  for (const [suffix, secondsInUnit] of RELATIVE_UNITS) {
    if (absSeconds >= secondsInUnit) {
      const value = Math.floor(absSeconds / secondsInUnit)
      return diffSeconds > 0 ? `in ${value}${suffix}` : `${value}${suffix} ago`
    }
  }

  return 'just now'
}
