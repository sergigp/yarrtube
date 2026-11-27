import { Star } from 'lucide-react'
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
  /** Draws a filled star: the video is a favorite. */
  favorite?: boolean
  /** Draws a "Kept" badge: a favorite its source keeps though it no longer lists it. */
  kept?: boolean
}

/** Badge placement per size: clear of the progress bar when one is shown. */
const BADGE_CLASSES: Record<ThumbnailSize, { flush: string; raised: string }> = {
  card: {
    flush: 'right-2 bottom-2 rounded-md px-1.5 text-xs font-semibold',
    raised: 'right-2 bottom-2.5 rounded-md px-1.5 text-xs font-semibold',
  },
  row: {
    flush: 'right-1 bottom-1 rounded px-1 text-[10px] font-medium',
    raised: 'right-1 bottom-2 rounded px-1 text-[10px] font-medium',
  },
}

/**
 * The watch state drawn over a video thumbnail: the watched tick, and either
 * the progress bar with the time left while the video is in progress or its
 * duration otherwise; and, top right, the favorite star and "Kept" badge.
 * Its parent must be positioned (`relative`).
 */
export function ThumbnailOverlay({
  watched,
  positionSeconds,
  durationSeconds,
  size,
  favorite = false,
  kept = false,
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
      <FavoriteMarks favorite={favorite} kept={kept} size={size} />
      {progress && <WatchProgressBar percent={progress.percent} size={size} />}
      {badge && (
        <span
          className={cn(
            'absolute bg-black/80 py-0.5 text-white',
            BADGE_CLASSES[size][progress ? 'raised' : 'flush'],
          )}
        >
          {badge}
        </span>
      )}
    </>
  )
}

/** The favorite star and "Kept" badge, top right; nothing for other videos. */
function FavoriteMarks({
  favorite,
  kept,
  size,
}: {
  favorite: boolean
  kept: boolean
  size: ThumbnailSize
}) {
  if (!favorite && !kept) {
    return null
  }
  const card = size === 'card'
  return (
    <span
      className={cn(
        'absolute flex items-center gap-1 text-white',
        card ? 'top-2 right-2' : 'top-1 right-1',
      )}
    >
      {kept && (
        <span
          className={cn(
            'rounded-md bg-black/80 font-semibold',
            card ? 'px-1.5 py-0.5 text-xs' : 'px-1 text-[10px]',
          )}
        >
          Kept
        </span>
      )}
      {favorite && (
        <span
          role="img"
          aria-label="Favorite"
          title="Favorite"
          className={cn(
            'flex items-center justify-center rounded-md bg-black/80',
            card ? 'size-6' : 'size-4',
          )}
        >
          <Star className={cn('fill-amber-400 text-amber-400', card ? 'size-3.5' : 'size-2.5')} />
        </span>
      )}
    </span>
  )
}
