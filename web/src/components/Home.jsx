import { Link } from 'react-router-dom'
import { usePolling } from '../usePolling'
import { fetchRecentVideos, fetchChannels, fetchPlaylists, videoMediaUrl } from '../api'
import { Thumbnail } from './Thumbnail'
import { cn } from '@/lib/utils'

function videoDetailPath(source) {
  const base = source.kind === 'channel' ? `/channels/${source.id}` : `/playlists/${source.id}`
  return `${base}?video=`
}

function VideoGrid({ videos }) {
  if (videos.length === 0) {
    return <p className="text-sm text-muted-foreground">No videos synced yet.</p>
  }

  return (
    <ul className="grid grid-cols-2 gap-x-4 gap-y-6 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5">
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
            <Thumbnail
              src={
                video.thumbnail_filename
                  ? videoMediaUrl(video.source.path, video.thumbnail_filename)
                  : null
              }
              className="aspect-video w-full rounded-lg object-cover transition-opacity group-hover:opacity-90"
            />
            <span className="line-clamp-2 text-sm leading-snug font-medium text-foreground">
              {video.title}
            </span>
          </Link>
        </li>
      ))}
    </ul>
  )
}

function SidebarSection({ title, items, error, hrefFor }) {
  return (
    <div>
      <h3 className="mb-2 text-sm font-medium text-muted-foreground">{title}</h3>
      {error && <p className="text-sm text-destructive">Failed to load: {error.message}</p>}
      {!error && !items && <p className="text-sm text-muted-foreground">Loading…</p>}
      {!error && items && items.length === 0 && (
        <p className="text-sm text-muted-foreground">None tracked yet.</p>
      )}
      {!error && items && items.length > 0 && (
        <ul className="flex flex-col">
          {items.map((item, index) => (
            <li
              key={item.id}
              className="animate-enter"
              style={{ animationDelay: `${Math.min(index, 10) * 25}ms` }}
            >
              <Link
                className={cn(
                  'block truncate rounded-md px-2 py-1.5 text-sm text-foreground no-underline',
                  '-mx-2 transition-colors hover:bg-accent hover:text-primary',
                )}
                to={hrefFor(item)}
              >
                {item.name}
              </Link>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}

export function Home() {
  const { data: videos, error: videosError } = usePolling(fetchRecentVideos, [])
  const { data: channels, error: channelsError } = usePolling(fetchChannels, [])
  const { data: playlists, error: playlistsError } = usePolling(fetchPlaylists, [])

  return (
    <div className="grid grid-cols-1 items-start gap-8 md:grid-cols-[220px_minmax(0,1fr)]">
      <aside className="flex flex-col gap-6">
        <SidebarSection
          title="Channels"
          items={channels}
          error={channelsError}
          hrefFor={(channel) => `/channels/${channel.id}`}
        />
        <SidebarSection
          title="Playlists"
          items={playlists}
          error={playlistsError}
          hrefFor={(playlist) => `/playlists/${playlist.id}`}
        />
      </aside>

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
    </div>
  )
}
