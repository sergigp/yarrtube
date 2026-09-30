interface WatchProgressBarProps {
  positionSeconds: number
  durationSeconds: number | null
}

/**
 * A thin bar along the bottom of a video thumbnail showing how far the video
 * has been watched, as its saved position over its duration. Renders nothing
 * without a known duration. Its parent must be positioned (`relative`).
 */
export function WatchProgressBar({ positionSeconds, durationSeconds }: WatchProgressBarProps) {
  if (!durationSeconds || durationSeconds <= 0) {
    return null
  }

  const percent = Math.min(100, Math.max(0, (positionSeconds / durationSeconds) * 100))

  return (
    <div
      className="absolute inset-x-0 bottom-0 h-1 overflow-hidden rounded-b-lg bg-white/30"
      role="progressbar"
      aria-label="Watch progress"
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={Math.round(percent)}
    >
      <div className="h-full bg-primary" style={{ width: `${percent}%` }} />
    </div>
  )
}
