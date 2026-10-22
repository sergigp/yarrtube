export interface WatchProgress {
  /** 0..100, saved position over duration. */
  percent: number
  /** Duration minus saved position, >= 0. */
  secondsLeft: number
}

/**
 * How far a video has been watched, when it's in progress: unwatched, with a
 * saved position past 0 and a known duration. Otherwise null.
 */
export function watchProgress(video: {
  watched: boolean
  position_seconds: number
  duration_seconds: number | null
}): WatchProgress | null {
  const duration = video.duration_seconds
  if (video.watched || video.position_seconds <= 0 || !duration || duration <= 0) {
    return null
  }

  const position = Math.min(video.position_seconds, duration)
  return {
    percent: (position / duration) * 100,
    secondsLeft: duration - position,
  }
}

/** Formats seconds left as "<1m left", "8m left", "1h left" or "1h 5m left", minutes rounded down. */
export function formatTimeLeft(seconds: number): string {
  const totalMinutes = Math.floor(seconds / 60)
  if (totalMinutes < 1) {
    return '<1m left'
  }

  const hours = Math.floor(totalMinutes / 60)
  const minutes = totalMinutes % 60
  if (hours === 0) {
    return `${minutes}m left`
  }
  return minutes === 0 ? `${hours}h left` : `${hours}h ${minutes}m left`
}
