import { TriangleAlert } from 'lucide-react'
import { videoMediaUrl } from '@/api/client'
import type { Video, VideoStatus } from '@/api/types'
import { formatDuration } from '@/lib/formatDuration'
import { Thumbnail } from './Thumbnail'
import { WatchedTick } from './WatchedTick'
import { Beacon } from './Beacon'
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
    <TriangleAlert className="size-3.5 shrink-0 text-amber-600" role="img" aria-label={message}>
      <title>{message}</title>
    </TriangleAlert>
  )
}

interface VideoListPaneProps {
  /** Names the container in the empty message ("channel" or "playlist"). */
  noun: string
  /** The channel/playlist path the thumbnails are served under. */
  basePath: string
  videos: Video[] | undefined
  error: Error | null
  selectedVideoId: string | null
  onSelect: (id: string) => void
}

/**
 * The scrollable video list beside a detail view's player: one row per
 * video with its thumbnail, watched tick, duration and download status,
 * highlighting the selected one.
 */
export function VideoListPane({
  noun,
  basePath,
  videos,
  error,
  selectedVideoId,
  onSelect,
}: VideoListPaneProps) {
  return (
    <div className="md:h-full md:min-h-0 md:overflow-y-auto">
      {error && <p className="text-sm text-destructive">Failed to load videos: {error.message}</p>}
      {!error && !videos && <p className="text-sm text-muted-foreground">Loading videos…</p>}
      {!error && videos && videos.length === 0 && (
        <p className="text-sm text-muted-foreground">No videos recorded for this {noun} yet.</p>
      )}
      {!error && videos && videos.length > 0 && (
        <ul className="flex flex-col divide-y divide-border">
          {videos.map((video) => {
            const active = selectedVideoId === video.id
            return (
              <li key={video.id}>
                <button
                  className={cn(
                    'flex w-full items-start gap-3 rounded-md px-2 py-2.5 text-left transition-colors hover:bg-accent',
                    active && 'bg-accent',
                  )}
                  onClick={() => onSelect(video.id)}
                >
                  <div className="relative shrink-0">
                    <Thumbnail
                      src={
                        video.thumbnail_filename
                          ? videoMediaUrl(basePath, video.thumbnail_filename)
                          : null
                      }
                      className="aspect-video w-24 rounded-md object-cover"
                    />
                    <WatchedTick watched={video.watched} />
                    {formatDuration(video.duration_seconds) && (
                      <span className="absolute right-1 bottom-1 rounded bg-black/75 px-1 py-0.5 text-[10px] font-medium text-white">
                        {formatDuration(video.duration_seconds)}
                      </span>
                    )}
                  </div>
                  <span
                    className={cn(
                      'min-w-0 flex-1 text-sm leading-snug text-foreground',
                      active && 'font-medium text-primary',
                    )}
                  >
                    {video.title}
                  </span>
                  <VideoStatusIndicator status={video.status} />
                </button>
              </li>
            )
          })}
        </ul>
      )}
    </div>
  )
}
