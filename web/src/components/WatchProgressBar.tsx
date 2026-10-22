import type { ThumbnailSize } from './ThumbnailOverlay'

interface WatchProgressBarProps {
  /** 0..100. */
  percent: number
  size: ThumbnailSize
}

/**
 * A bar along the bottom of a video thumbnail showing how far the video has
 * been watched. Its parent must be positioned (`relative`).
 */
export function WatchProgressBar({ percent, size }: WatchProgressBarProps) {
  void size
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
