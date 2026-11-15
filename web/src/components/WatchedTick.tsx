import { Check } from 'lucide-react'
import { cn } from '@/lib/utils'
import type { ThumbnailSize } from './ThumbnailOverlay'

/** Bottom offset on rows: clear of the progress bar when raised. */
const ROW_CLASSES = { flush: 'bottom-1 left-1', raised: 'bottom-2 left-1' }

interface WatchedTickProps {
  watched: boolean
  size: ThumbnailSize
  /** Lifts a row's tick above a progress bar along the thumbnail's bottom edge. */
  raised: boolean
}

/**
 * Marks a watched video's thumbnail: a labelled "Watched" badge in a card's
 * top-left corner, a bare tick in a list row's bottom-left one. Its parent
 * must be positioned (`relative`).
 */
export function WatchedTick({ watched, size, raised }: WatchedTickProps) {
  if (!watched) {
    return null
  }

  if (size === 'card') {
    return (
      <span
        className="absolute top-2 left-2 flex h-6 items-center gap-1 rounded-md bg-black/80 px-2 text-xs font-semibold text-white"
        role="img"
        aria-label="Watched"
        title="Watched"
      >
        <Check className="size-3" strokeWidth={3} />
        Watched
      </span>
    )
  }

  return (
    <span
      className={cn(
        'absolute flex size-5 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-sm',
        ROW_CLASSES[raised ? 'raised' : 'flush'],
      )}
      role="img"
      aria-label="Watched"
      title="Watched"
    >
      <Check className="size-3.5" strokeWidth={3} />
    </span>
  )
}
