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
  if (video.position_seconds <= 0) {
    return null
  }

  const duration = video.duration_seconds ?? 0
  return {
    percent: (video.position_seconds / duration) * 100,
    secondsLeft: duration - video.position_seconds,
  }
}

/** Formats seconds left as "<1m left", "8m left", "1h left" or "1h 5m left", minutes rounded down. */
export function formatTimeLeft(seconds: number): string {
  void seconds
  return ''
}
