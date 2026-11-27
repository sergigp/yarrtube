import { Play } from 'lucide-react'
import { Link } from 'react-router-dom'
import { videoMediaUrl, avatarMediaUrl } from '@/api/client'
import type { HomeVideo } from '@/api/types'
import { formatAge } from '@/lib/relativeAge'
import { videoPathInSource } from '@/lib/videoPath'
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
import { CardMarkWatchedButton } from './CardMarkWatchedButton'
import { PlaylistIcon } from './PlaylistIcon'
import { channelPath, playlistPath } from '@/lib/sourcePath'

interface VideoGridProps {
  videos: HomeVideo[]
  /** Category id → chip; each card shows its categories' chips. Absent shows none. */
  chips?: Map<string, CategoryChip>
  /** How the cards are laid out; a wrapping grid by default. */
  layout?: VideoGridLayout
  /** Where a card leads; absent, to the video in its channel or playlist. */
  videoPathOf?: (video: HomeVideo) => string
  /**
   * Replaces the source link of the meta line, e.g. "Ep 4" on a playlist's
   * own page, where naming the playlist again says nothing.
   */
  labelOf?: (video: HomeVideo) => string | null
  /** Whether a card's video is kept by its source, which badges it "Kept". */
  keptOf?: (video: HomeVideo) => boolean
}

/** Home-style video cards, each linking to its video in its source. */
export function VideoGrid({
  videos,
  chips,
  layout = 'grid',
  videoPathOf,
  labelOf,
  keptOf,
}: VideoGridProps) {
  if (videos.length === 0) {
    return <p className="text-sm text-muted-foreground">No videos synced yet.</p>
  }

  return (
    <ul className={videoGridListClassName(layout)}>
      {videos.map((video, index) => {
        const videoPath = videoPathOf?.(video) ?? videoPathInSource(video)
        const sourcePath =
          video.source.kind === 'channel'
            ? channelPath(video.source.id)
            : playlistPath(video.source.id)
        const age = formatAge(video.published_at)
        return (
          <li
            key={`${video.source.kind}:${video.source.id}:${video.id}`}
            // Frameless. The title's link is stretched over the whole card, so a
            // click anywhere opens the video; the source link, the quick action
            // and the menu sit above it (`z-10`).
            className={cn(
              'group/card animate-enter relative min-w-0',
              videoGridItemClassName(layout, index),
            )}
            style={{ animationDelay: `${Math.min(index, 12) * 35}ms` }}
          >
            <div className="flex flex-col gap-2.5">
              <div
                className="relative overflow-hidden rounded-xl bg-muted"
                data-slot="card-thumbnail"
              >
                <Thumbnail
                  src={
                    video.thumbnail_filename
                      ? videoMediaUrl(video.source.path, video.thumbnail_filename)
                      : null
                  }
                  className={cn(
                    'block aspect-video w-full object-cover transition-[opacity,transform] duration-300 group-hover/card:scale-[1.03]',
                    video.watched && 'opacity-45 group-hover/card:opacity-80',
                  )}
                />
                <PlayHint />
                <ThumbnailOverlay
                  watched={video.watched}
                  positionSeconds={video.position_seconds}
                  durationSeconds={video.duration_seconds}
                  size="card"
                  favorite={video.favorite}
                  kept={keptOf?.(video) ?? false}
                />
                {!video.watched && <CardMarkWatchedButton videoId={video.id} title={video.title} />}
              </div>
              <div className="flex items-start gap-1">
                <div className="flex min-w-0 flex-1 flex-col gap-1">
                  <Link
                    to={videoPath}
                    className={cn(
                      "line-clamp-2 text-sm leading-snug font-semibold no-underline after:absolute after:inset-0 after:content-[''] sm:text-[0.9375rem]",
                      video.watched ? 'text-muted-foreground' : 'text-foreground',
                    )}
                  >
                    {video.title}
                  </Link>
                  {/* The source's name truncates first, so the age always shows whole. */}
                  <p
                    data-slot="card-meta"
                    className="flex min-w-0 items-center text-[0.8125rem] text-muted-foreground"
                  >
                    {labelOf ? (
                      <MetaLabel label={labelOf(video)} />
                    ) : (
                      <Link
                        to={sourcePath}
                        className="relative z-10 flex min-w-0 items-center gap-1.5 no-underline transition-colors hover:text-foreground"
                      >
                        {video.source.kind === 'channel' ? (
                          <Thumbnail
                            src={
                              video.source.avatar_filename
                                ? avatarMediaUrl(video.source.avatar_filename)
                                : null
                            }
                            className="block size-4.5 shrink-0 rounded-full object-cover"
                          />
                        ) : (
                          <PlaylistIcon className="size-4.5 rounded-[5px]" />
                        )}
                        <span className="truncate">{video.source.name}</span>
                      </Link>
                    )}
                    {age && (
                      <span className="shrink-0 whitespace-pre">
                        {labelOf && !labelOf(video) ? age : ` · ${age}`}
                      </span>
                    )}
                  </p>
                  {chips && <CategoryChips categoryIds={video.category_ids} chips={chips} />}
                </div>
                <VideoActionsMenu
                  videoId={video.id}
                  title={video.title}
                  sourceName={video.source.name}
                  markable={!video.watched}
                  discard={{ downloaded: true }}
                  favorite={{ favorite: video.favorite }}
                  excludablePlaylist={
                    video.source.kind === 'playlist'
                      ? { id: video.source.id, name: video.source.name }
                      : undefined
                  }
                  className={cn(
                    'relative z-10 -mt-1 -mr-1.5',
                    // Only on hover or focus where the device can hover; always on touch.
                    '[@media(hover:hover)]:opacity-0 [@media(hover:hover)]:group-hover/card:opacity-100 [@media(hover:hover)]:group-has-[:focus-visible]/card:opacity-100 [@media(hover:hover)]:data-[state=open]:opacity-100',
                  )}
                />
              </div>
            </div>
          </li>
        )
      })}
    </ul>
  )
}

/** A play icon over the thumbnail while the card is hovered, under a dimming scrim. */
function PlayHint() {
  return (
    <span
      aria-hidden="true"
      className="pointer-events-none absolute inset-0 flex items-center justify-center bg-black/35 opacity-0 transition-opacity group-hover/card:opacity-100"
    >
      <span className="flex size-12 items-center justify-center rounded-full bg-black/70 text-white">
        <Play className="ml-0.5 size-5 fill-current" />
      </span>
    </span>
  )
}

/** The meta line's own label in place of the source, e.g. "Ep 4". */
function MetaLabel({ label }: { label: string | null }) {
  return label ? <span className="truncate">{label}</span> : null
}
