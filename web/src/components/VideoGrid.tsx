import { ListVideo } from 'lucide-react'
import { Link } from 'react-router-dom'
import { videoMediaUrl, avatarMediaUrl } from '@/api/client'
import type { HomeVideo, HomeVideoSource } from '@/api/types'
import { formatAge } from '@/lib/relativeAge'
import {
  type VideoGridLayout,
  videoGridItemClassName,
  videoGridListClassName,
} from '@/lib/videoGridLayout'
import { cn } from '@/lib/utils'
import { Thumbnail } from './Thumbnail'
import { CategoryChips, type CategoryChip } from './CategoryChips'
import { ThumbnailOverlay } from './ThumbnailOverlay'
import { VideoActionsMenu } from './VideoActionsMenu'

function videoDetailPath(source: HomeVideoSource): string {
  const base = source.kind === 'channel' ? `/channels/${source.id}` : `/playlists/${source.id}`
  return `${base}?video=`
}

interface VideoGridProps {
  videos: HomeVideo[]
  /** Category id → chip; each card shows its categories' chips. Absent shows none. */
  chips?: Map<string, CategoryChip>
  /** How the cards are laid out; a wrapping grid by default. */
  layout?: VideoGridLayout
  /** Where a card leads; absent, to the video in its channel or playlist. */
  videoPathOf?: (video: HomeVideo) => string
}

/** Home-style video cards, each linking to its video in its source. */
export function VideoGrid({ videos, chips, layout = 'grid', videoPathOf }: VideoGridProps) {
  if (videos.length === 0) {
    return <p className="text-sm text-muted-foreground">No videos synced yet.</p>
  }

  return (
    <ul className={videoGridListClassName(layout)}>
      {videos.map((video, index) => {
        const videoPath =
          videoPathOf?.(video) ?? `${videoDetailPath(video.source)}${encodeURIComponent(video.id)}`
        const channelPath = video.source.kind === 'channel' ? `/channels/${video.source.id}` : null
        const sourcePath = channelPath ?? `/playlists/${video.source.id}`
        const age = formatAge(video.published_at)
        return (
          <li
            key={`${video.source.kind}:${video.source.id}:${video.id}`}
            // Frameless: the hover surface reaches past the content through the
            // negative margin, so it never shifts the card.
            className={cn(
              'group/card animate-enter -m-2 rounded-xl p-2 transition-colors hover:bg-muted/60',
              videoGridItemClassName(layout, index),
            )}
            style={{ animationDelay: `${Math.min(index, 12) * 35}ms` }}
          >
            <div className="flex flex-col gap-2">
              <Link
                className="group relative block overflow-hidden rounded-lg"
                to={videoPath}
                data-slot="card-thumbnail"
              >
                <Thumbnail
                  src={
                    video.thumbnail_filename
                      ? videoMediaUrl(video.source.path, video.thumbnail_filename)
                      : null
                  }
                  className={cn(
                    'aspect-video w-full rounded-lg object-cover transition-[opacity,transform] duration-300 group-hover/card:scale-[1.03]',
                    video.watched && 'opacity-60 group-hover/card:opacity-100',
                  )}
                />
                <ThumbnailOverlay
                  watched={video.watched}
                  positionSeconds={video.position_seconds}
                  durationSeconds={video.duration_seconds}
                  size="card"
                />
              </Link>
              <div className="flex items-start gap-2">
                {channelPath ? (
                  <Link to={channelPath} className="mt-0.5 shrink-0" aria-label={video.source.name}>
                    <Thumbnail
                      src={
                        video.source.avatar_filename
                          ? avatarMediaUrl(video.source.avatar_filename)
                          : null
                      }
                      className="block size-6 rounded-full object-cover"
                    />
                  </Link>
                ) : (
                  // Stands in for a channel's avatar, so titles line up across cards.
                  <span className="mt-0.5 flex size-6 shrink-0 items-center justify-center rounded-full bg-muted text-muted-foreground">
                    <ListVideo role="img" aria-label="Playlist" className="size-3.5" />
                  </span>
                )}
                <div className="flex min-w-0 flex-1 flex-col gap-0.5">
                  <Link
                    to={videoPath}
                    className={cn(
                      'line-clamp-2 text-sm leading-snug font-medium no-underline',
                      video.watched ? 'text-muted-foreground' : 'text-foreground',
                    )}
                  >
                    {video.title}
                  </Link>
                  <p data-slot="card-meta" className="truncate text-xs text-muted-foreground">
                    <Link
                      to={sourcePath}
                      className="no-underline transition-colors hover:text-foreground"
                    >
                      {video.source.name}
                    </Link>
                    {age && ` · ${age}`}
                  </p>
                  {chips && <CategoryChips categoryIds={video.category_ids} chips={chips} />}
                </div>
                <VideoActionsMenu
                  videoId={video.id}
                  title={video.title}
                  markable={!video.watched}
                  excludablePlaylist={
                    video.source.kind === 'playlist'
                      ? { id: video.source.id, name: video.source.name }
                      : undefined
                  }
                  className="-mt-0.5 -mr-1.5"
                />
              </div>
            </div>
          </li>
        )
      })}
    </ul>
  )
}
