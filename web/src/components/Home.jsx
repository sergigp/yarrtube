import { Link } from 'react-router-dom'
import { usePolling } from '../usePolling'
import { fetchRecentVideos, videoMediaUrl, avatarMediaUrl } from '../api'
import { formatDuration } from '../formatDuration'
import { Thumbnail } from './Thumbnail'

function videoDetailPath(source) {
  const base = source.kind === 'channel' ? `/channels/${source.id}` : `/playlists/${source.id}`
  return `${base}?video=`
}

function VideoGrid({ videos }) {
  if (videos.length === 0) {
    return <p className="text-sm text-muted-foreground">No videos synced yet.</p>
  }

  return (
    <ul className="grid grid-cols-2 gap-x-4 gap-y-6 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6">
      {videos.map((video, index) => (
        <li
          key={`${video.source.kind}:${video.source.id}:${video.id}`}
          className="animate-enter"
          style={{ animationDelay: `${Math.min(index, 12) * 35}ms` }}
        >
          <Link
            className="group flex flex-col gap-2 no-underline"
            to={`${videoDetailPath(video.source)}${encodeURIComponent(video.id)}`}
          >
            <div className="relative">
              <Thumbnail
                src={
                  video.thumbnail_filename
                    ? videoMediaUrl(video.source.path, video.thumbnail_filename)
                    : null
                }
                className="aspect-video w-full rounded-lg object-cover transition-opacity group-hover:opacity-90"
              />
              {formatDuration(video.duration_seconds) && (
                <span className="absolute right-1.5 bottom-1.5 rounded bg-black/75 px-1.5 py-0.5 text-xs font-medium text-white">
                  {formatDuration(video.duration_seconds)}
                </span>
              )}
            </div>
            <div className="flex items-start gap-2">
              {video.source.kind === 'channel' && (
                <Thumbnail
                  src={
                    video.source.avatar_filename
                      ? avatarMediaUrl(video.source.avatar_filename)
                      : null
                  }
                  className="mt-0.5 size-6 shrink-0 rounded-full object-cover"
                />
              )}
              <span className="line-clamp-2 text-sm leading-snug font-medium text-foreground">
                {video.title}
              </span>
            </div>
          </Link>
        </li>
      ))}
    </ul>
  )
}

export function Home() {
  const { data: videos, error: videosError } = usePolling(fetchRecentVideos, [])

  return (
    <div className="min-w-0">
      <h2 className="mb-4 font-heading text-lg font-semibold text-foreground">Latest videos</h2>
      {videosError && (
        <p className="text-sm text-destructive">
          Failed to load recent videos: {videosError.message}
        </p>
      )}
      {!videosError && !videos && (
        <p className="text-sm text-muted-foreground">Loading recent videos…</p>
      )}
      {!videosError && videos && <VideoGrid videos={videos} />}
    </div>
  )
}
