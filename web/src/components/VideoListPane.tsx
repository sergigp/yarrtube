import { TriangleAlert } from 'lucide-react'
import { videoMediaUrl } from '@/api/client'
import type { Video, VideoStatus } from '@/api/types'
import { Thumbnail } from './Thumbnail'
import { ThumbnailOverlay } from './ThumbnailOverlay'
import { Beacon } from './Beacon'
import { VideoActionsMenu } from './VideoActionsMenu'
import { VideoRowsSkeleton } from './VideoSkeletons'
import { formatAge } from '@/lib/relativeAge'
import { cn } from '@/lib/utils'

const STATUS_MESSAGES: Record<string, string> = {
  PENDING: 'This video is pending.',
  ERRORED_RETRYING: 'This video failed to download and will be retried.',
  ERRORED: 'This video failed to download.',
}

function VideoStatusIndicator({ status }: { status: VideoStatus }) {
  if (status === 'DOWNLOADED') {
    return null
  }

  if (status === 'IN_PROGRESS') {
    return <Beacon variant="live" label="Downloading" className="shrink-0" />
  }

  const message = STATUS_MESSAGES[status] ?? 'This video is pending.'
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
  videos: T[] | undefined
  error: Error | null
  selectedVideoId: string | null
  onSelect: (id: string) => void
}

/**
 * The scrollable video list beside a detail view's player: one row per
 * video with its thumbnail, watch state, duration, download status and
 * actions menu, highlighting the selected one. The menu sits beside the
 * row's select button, not inside it, so opening it never selects the video;
 * the selected row has none, since its detail pane shows one.
 */
export function VideoListPane<T extends Video>({
  noun,
  basePathOf,
  videos,
  error,
  selectedVideoId,
  onSelect,
}: VideoListPaneProps<T>) {
  return (
    <div className="md:h-full md:min-h-0 md:overflow-y-auto">
      {error && <p className="text-sm text-destructive">Failed to load videos: {error.message}</p>}
      {!error && !videos && <VideoRowsSkeleton count={8} />}
      {!error && videos && videos.length === 0 && (
        <p className="text-sm text-muted-foreground">No videos recorded for this {noun} yet.</p>
      )}
      {!error && videos && videos.length > 0 && (
        <ul className="flex flex-col">
          {videos.map((video) => {
            const active = selectedVideoId === video.id
            const age = formatAge(video.published_at)
            return (
              <li
                key={video.id}
                className={cn(
                  // The separator is a straight line, not a border: a border
                  // on the rounded row would curl up at its corners.
                  'relative flex items-start gap-1 rounded-md pr-1 transition-colors not-last:after:absolute not-last:after:inset-x-2 not-last:after:bottom-0 not-last:after:h-px not-last:after:bg-border hover:bg-accent',
                  active && 'bg-accent',
                )}
              >
                <button
                  className="flex min-w-0 flex-1 items-start gap-3 py-2.5 pl-2 text-left"
                  onClick={() => onSelect(video.id)}
                >
                  <div className="relative shrink-0">
                    <Thumbnail
                      src={
                        video.thumbnail_filename
                          ? videoMediaUrl(basePathOf(video), video.thumbnail_filename)
                          : null
                      }
                      className={cn(
                        'aspect-video w-24 rounded-md object-cover',
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
                        'text-sm leading-snug',
                        active
                          ? 'font-medium text-primary'
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
                {active ? (
                  // The detail pane holds the selected video's menu; the
                  // placeholder keeps the row's title from re-wrapping.
                  <span aria-hidden className="mt-2 size-7 shrink-0" />
                ) : (
                  <VideoActionsMenu
                    videoId={video.id}
                    title={video.title}
                    markable={!video.watched && video.status === 'DOWNLOADED'}
                    className="mt-2"
                  />
                )}
              </li>
            )
          })}
        </ul>
      )}
    </div>
  )
}
