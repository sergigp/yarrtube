import { TriangleAlert } from 'lucide-react'
import { videoMediaUrl } from '@/api/client'
import type { Video, VideoStatus } from '@/api/types'
import { Thumbnail } from './Thumbnail'
import { ThumbnailOverlay } from './ThumbnailOverlay'
import { Beacon } from './Beacon'
import { VideoActionsMenu } from './VideoActionsMenu'
import { VideoRowsSkeleton } from './VideoSkeletons'
import { Switch } from '@/components/ui/switch'
import { useRememberedFlag } from '@/hooks/useRememberedFlag'
import { formatAge } from '@/lib/relativeAge'
import { cn } from '@/lib/utils'

/** Where "Hide watched" is remembered, for every queue alike. */
export const HIDE_WATCHED_KEY = 'yarrtube.queue.hideWatched'

const STATUS_MESSAGES: Record<Exclude<VideoStatus, 'DOWNLOADED' | 'IN_PROGRESS'>, string> = {
  PENDING: 'This video is pending.',
  RETRYING: 'This video failed to download and will be retried.',
  FAILED: 'This video failed to download.',
  UNAVAILABLE: 'This video is unavailable on YouTube.',
}

function VideoStatusIndicator({ status }: { status: VideoStatus }) {
  if (status === 'DOWNLOADED') {
    return null
  }

  if (status === 'IN_PROGRESS') {
    return <Beacon variant="live" label="Downloading" className="shrink-0" />
  }

  const message = STATUS_MESSAGES[status]
  return (
    <TriangleAlert className="size-3.5 shrink-0 text-amber-600 dark:text-amber-400" role="img" aria-label={message}>
      <title>{message}</title>
    </TriangleAlert>
  )
}

interface VideoListPaneProps<T extends Video> {
  /** Names the container in the empty message ("channel", "playlist" or "category"). */
  noun: string
  /** The channel/playlist path a video's thumbnail is served under. */
  basePathOf: (video: T) => string
  /** The name of the channel or playlist a video comes from. */
  sourceNameOf: (video: T) => string
  videos: T[] | undefined
  error: Error | null
  selectedVideoId: string | null
  onSelect: (id: string) => void
  /** A playlist's episode number per video, shown before its row. */
  episodeOf?: (video: T) => number | null
  /** How the list is ordered, after the position, e.g. "playlist order". */
  orderLabel?: string
}

/**
 * The queue beside a detail view's player: an "Up next" header with the
 * selected video's position and a "Hide watched" switch (remembered in this
 * browser; the selected video always stays), then one row per video with
 * its episode number in a playlist, thumbnail, watch state, duration,
 * download status and actions menu, highlighting the selected one. The menu
 * sits beside the row's select button, not inside it, so opening it never
 * selects the video.
 */
export function VideoListPane<T extends Video>({
  noun,
  basePathOf,
  sourceNameOf,
  videos,
  error,
  selectedVideoId,
  onSelect,
  episodeOf,
  orderLabel,
}: VideoListPaneProps<T>) {
  const [hideWatched, toggleHideWatched] = useRememberedFlag(HIDE_WATCHED_KEY, false)
  const position = videos ? videos.findIndex((video) => video.id === selectedVideoId) : -1
  const shown = hideWatched
    ? videos?.filter((video) => !video.watched || video.id === selectedVideoId)
    : videos
  const positionLine = [
    videos && videos.length > 0 && position >= 0 ? `${position + 1} of ${videos.length}` : null,
    orderLabel,
  ]
    .filter(Boolean)
    .join(' · ')

  return (
    <aside
      aria-label="Queue"
      className="flex flex-col rounded-xl bg-secondary/70 md:h-full md:min-h-0 md:overflow-y-auto dark:bg-card"
    >
      <div className="sticky top-0 z-[1] flex items-center justify-between gap-2 rounded-t-xl border-b border-border bg-inherit px-4 py-3 max-md:static">
        <div className="flex min-w-0 flex-col">
          <h3 className="font-semibold text-foreground">Up next</h3>
          {positionLine && (
            <span className="truncate text-xs text-muted-foreground">{positionLine}</span>
          )}
        </div>
        <label className="flex shrink-0 items-center gap-2 text-[0.8125rem] text-muted-foreground">
          Hide watched
          <Switch checked={hideWatched} onCheckedChange={toggleHideWatched} />
        </label>
      </div>
      <div className="p-2">
      {error && <p className="p-2 text-sm text-destructive">Failed to load videos: {error.message}</p>}
      {!error && !videos && <VideoRowsSkeleton count={8} />}
      {!error && videos && videos.length === 0 && (
        <p className="p-2 text-sm text-muted-foreground">No videos recorded for this {noun} yet.</p>
      )}
      {!error && shown && videos && videos.length > 0 && shown.length === 0 && (
        <p className="p-2 text-sm text-muted-foreground">Every video is watched.</p>
      )}
      {!error && shown && shown.length > 0 && (
        <ul className="flex flex-col gap-0.5">
          {shown.map((video) => {
            const active = selectedVideoId === video.id
            const age = formatAge(video.published_at)
            const episode = episodeOf?.(video) ?? null
            return (
              <li
                key={video.id}
                className={cn(
                  'relative flex items-start gap-1 rounded-[10px] pr-1 transition-colors hover:bg-background dark:hover:bg-accent',
                  active && 'bg-accent!',
                )}
              >
                <button
                  className="flex min-w-0 flex-1 items-start gap-2.5 py-2 pl-2 text-left"
                  aria-current={active ? 'true' : undefined}
                  onClick={() => onSelect(video.id)}
                >
                  {episodeOf && (
                    <span
                      className={cn(
                        'mt-6 w-6 shrink-0 text-right text-xs tabular-nums',
                        active ? 'text-primary' : 'text-muted-foreground',
                      )}
                    >
                      {episode !== null && <span className="sr-only">Episode </span>}
                      {episode}
                    </span>
                  )}
                  <div className="relative shrink-0">
                    <Thumbnail
                      src={
                        video.thumbnail_filename
                          ? videoMediaUrl(basePathOf(video), video.thumbnail_filename)
                          : null
                      }
                      className={cn(
                        'aspect-video w-28 rounded-lg object-cover',
                        video.watched && 'opacity-60',
                      )}
                    />
                    <ThumbnailOverlay
                      watched={video.watched}
                      positionSeconds={video.position_seconds}
                      durationSeconds={video.duration_seconds}
                      size="row"
                    />
                  </div>
                  <span className="flex min-w-0 flex-1 flex-col gap-0.5">
                    <span
                      className={cn(
                        'line-clamp-2 text-[0.8125rem] leading-snug font-semibold',
                        active
                          ? 'text-primary'
                          : video.watched
                            ? 'text-muted-foreground'
                            : 'text-foreground',
                      )}
                    >
                      {video.title}
                    </span>
                    {age && <span className="text-xs text-muted-foreground">{age}</span>}
                  </span>
                  <VideoStatusIndicator status={video.status} />
                </button>
                <VideoActionsMenu
                  videoId={video.id}
                  title={video.title}
                  sourceName={sourceNameOf(video)}
                  markable={!video.watched && video.status === 'DOWNLOADED'}
                  className="mt-2"
                />
              </li>
            )
          })}
        </ul>
      )}
      </div>
    </aside>
  )
}
