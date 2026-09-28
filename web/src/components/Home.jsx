import { Link } from 'react-router-dom'
import { usePolling } from '../usePolling'
import { fetchRecentVideos, videoMediaUrl, avatarMediaUrl } from '../api'
import { formatDuration } from '../formatDuration'
import { Thumbnail } from './Thumbnail'
import { WatchedTick } from './WatchedTick'

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
                  className="aspect-video w-full rounded-lg object-cover transition-opacity group-hover:opacity-90"
                />
                <WatchedTick watched={video.watched} />
                {formatDuration(video.duration_seconds) && (
                  <span className="absolute right-1.5 bottom-1.5 rounded bg-black/75 px-1.5 py-0.5 text-xs font-medium text-white">
                    {formatDuration(video.duration_seconds)}
                  </span>
                )}
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
                <div className="flex min-w-0 flex-col gap-0.5">
                  <Link
                    to={videoPath}
                    className="line-clamp-2 text-sm leading-snug font-medium text-foreground no-underline"
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
              </div>
            </div>
          </li>
        )
      })}
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
