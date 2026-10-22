import { Check } from 'lucide-react'
import { cn } from '@/lib/utils'
import type { ThumbnailSize } from './ThumbnailOverlay'

/** Bottom offset: clear of the progress bar when raised. */
const BOTTOM_CLASSES: Record<ThumbnailSize, { flush: string; raised: string }> = {
  card: { flush: 'bottom-1.5 left-1.5', raised: 'bottom-3 left-1.5' },
  row: { flush: 'bottom-1 left-1', raised: 'bottom-2 left-1' },
}

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
  if (!watched) {
    return null
  }

  return (
    <span
      className={cn(
        'absolute flex size-5 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-sm',
        BOTTOM_CLASSES[size][raised ? 'raised' : 'flush'],
      )}
      role="img"
      aria-label="Watched"
      title="Watched"
    >
      <Check className="size-3.5" strokeWidth={3} />
    </span>
  )
}
