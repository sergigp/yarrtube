import { Link } from 'react-router-dom'
import { videoMediaUrl, avatarMediaUrl } from '@/api/client'
import type { HomeVideo, HomeVideoSource } from '@/api/types'
import { cn } from '@/lib/utils'
import { Thumbnail } from './Thumbnail'
import { ThumbnailOverlay } from './ThumbnailOverlay'
import { VideoActionsMenu } from './VideoActionsMenu'

function videoDetailPath(source: HomeVideoSource): string {
  const base = source.kind === 'channel' ? `/channels/${source.id}` : `/playlists/${source.id}`
  return `${base}?video=`
}

/** Home-style video cards, each linking to its video in its source. */
export function VideoGrid({ videos }: { videos: HomeVideo[] }) {
  if (videos.length === 0) {
    return <p className="text-sm text-muted-foreground">No videos synced yet.</p>
  }

  return (
    <ul className="grid grid-cols-2 gap-x-4 gap-y-6 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6">
      {videos.map((video, index) => {
        const videoPath = `${videoDetailPath(video.source)}${encodeURIComponent(video.id)}`
        const channelPath = video.source.kind === 'channel' ? `/channels/${video.source.id}` : null
        return (
          <li
            key={`${video.source.kind}:${video.source.id}:${video.id}`}
            className="animate-enter"
            style={{ animationDelay: `${Math.min(index, 12) * 35}ms` }}
          >
            <div className="flex flex-col gap-2">
              <Link className="group relative block" to={videoPath}>
                <Thumbnail
                  src={
                    video.thumbnail_filename
                      ? videoMediaUrl(video.source.path, video.thumbnail_filename)
                      : null
                  }
                  className={cn(
                    'aspect-video w-full rounded-lg object-cover transition-opacity',
                    video.watched ? 'opacity-60 group-hover:opacity-100' : 'group-hover:opacity-90',
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
                {channelPath && (
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
                  {channelPath && (
                    <Link
                      to={channelPath}
                      className="truncate text-xs text-muted-foreground no-underline transition-colors hover:text-foreground"
                    >
                      {video.source.name}
                    </Link>
                  )}
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
