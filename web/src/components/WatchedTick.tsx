import { Check } from 'lucide-react'
import type { ThumbnailSize } from './ThumbnailOverlay'

interface WatchedTickProps {
  watched: boolean
  size: ThumbnailSize
  /** Lifts the tick above a progress bar along the thumbnail's bottom edge. */
  raised: boolean
}

/**
 * A tick overlaid on a video thumbnail when that video has been watched.
 * Its parent must be positioned (`relative`).
 */
export function WatchedTick({ watched, size, raised }: WatchedTickProps) {
  void size
  void raised
  if (!watched) {
    return null
  }

  return (
    <span
      className="absolute bottom-1 left-1 flex size-5 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-sm"
      role="img"
      aria-label="Watched"
      title="Watched"
    >
      <Check className="size-3.5" strokeWidth={3} />
    </span>
  )
}
