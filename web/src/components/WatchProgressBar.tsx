import { cn } from '@/lib/utils'
import type { ThumbnailSize } from './ThumbnailOverlay'

/** Thicker on cards; rounded to match each thumbnail's corners. */
const SIZE_CLASSES: Record<ThumbnailSize, string> = {
  card: 'h-1.5 rounded-b-lg',
  row: 'h-1 rounded-b-md',
}

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
  return (
    <div
      className={cn('absolute inset-x-0 bottom-0 overflow-hidden bg-black/50', SIZE_CLASSES[size])}
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
