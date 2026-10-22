import { formatDuration } from '@/lib/formatDuration'
import { formatTimeLeft, watchProgress } from '@/lib/watchProgress'
import { cn } from '@/lib/utils'
import { WatchedTick } from './WatchedTick'
import { WatchProgressBar } from './WatchProgressBar'

/** A home/category card thumbnail, or the smaller one of a video list row. */
export type ThumbnailSize = 'card' | 'row'

interface ThumbnailOverlayProps {
  watched: boolean
  positionSeconds: number
  durationSeconds: number | null
  size: ThumbnailSize
}

/** Badge placement per size: clear of the progress bar when one is shown. */
const BADGE_CLASSES: Record<ThumbnailSize, { flush: string; raised: string }> = {
  card: { flush: 'right-1.5 bottom-1.5 px-1.5 text-xs', raised: 'right-1.5 bottom-3 px-1.5 text-xs' },
  row: { flush: 'right-1 bottom-1 px-1 text-[10px]', raised: 'right-1 bottom-2 px-1 text-[10px]' },
}

/**
 * The watch state drawn over a video thumbnail: the watched tick, and either
 * the progress bar with the time left while the video is in progress or its
 * duration otherwise. Its parent must be positioned (`relative`).
 */
export function ThumbnailOverlay({
  watched,
  positionSeconds,
  durationSeconds,
  size,
}: ThumbnailOverlayProps) {
  const progress = watchProgress({
    watched,
    position_seconds: positionSeconds,
    duration_seconds: durationSeconds,
  })
  const badge = progress ? formatTimeLeft(progress.secondsLeft) : formatDuration(durationSeconds)

  return (
    <>
      <WatchedTick watched={watched} size={size} raised={progress !== null} />
      {progress && <WatchProgressBar percent={progress.percent} size={size} />}
      {badge && (
        <span
          className={cn(
            'absolute rounded bg-black/75 py-0.5 font-medium text-white',
            BADGE_CLASSES[size][progress ? 'raised' : 'flush'],
          )}
        >
          {badge}
        </span>
      )}
    </>
  )
}
