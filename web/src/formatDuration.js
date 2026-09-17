/**
 * Formats a duration in seconds as "12:34" or "1:02:03".
 */
export function formatDuration(seconds) {
  if (seconds == null || !Number.isFinite(seconds)) {
    return null
  }

  const total = Math.max(0, Math.round(seconds))
  const hours = Math.floor(total / 3600)
  const minutes = Math.floor((total % 3600) / 60)
  const secs = total % 60

  if (hours > 0) {
    return `${hours}:${String(minutes).padStart(2, '0')}:${String(secs).padStart(2, '0')}`
  }
  return `${minutes}:${String(secs).padStart(2, '0')}`
}
