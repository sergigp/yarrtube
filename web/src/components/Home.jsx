import { Link } from 'react-router-dom'
import { usePolling } from '../usePolling'
import { fetchRecentVideos, fetchChannels, fetchPlaylists, videoMediaUrl } from '../api'

function videoDetailPath(source) {
  const base = source.kind === 'channel' ? `/channels/${source.id}` : `/playlists/${source.id}`
  return `${base}?video=`
}

function VideoGrid({ videos }) {
  if (videos.length === 0) {
    return <p className="muted">No videos synced yet.</p>
  }

  return (
    <ul className="video-card-grid">
      {videos.map((video) => (
        <li key={`${video.source.kind}:${video.source.id}:${video.id}`}>
          <Link
            className="video-card"
            to={`${videoDetailPath(video.source)}${encodeURIComponent(video.id)}`}
          >
            {video.thumbnail_filename ? (
              <img
                className="video-card-thumbnail"
                src={videoMediaUrl(video.source.path, video.thumbnail_filename)}
                alt=""
              />
            ) : (
              <span className="video-card-thumbnail-placeholder" />
            )}
            <span className="video-card-title">{video.title}</span>
          </Link>
        </li>
      ))}
    </ul>
  )
}

function SidebarSection({ title, items, error, hrefFor }) {
  return (
    <div className="home-sidebar-section">
      <h3 className="home-sidebar-heading">{title}</h3>
      {error && <p className="error">Failed to load: {error.message}</p>}
      {!error && !items && <p className="muted">Loading…</p>}
      {!error && items && items.length === 0 && <p className="muted">None tracked yet.</p>}
      {!error && items && items.length > 0 && (
        <ul className="home-sidebar-list">
          {items.map((item) => (
            <li key={item.id}>
              <Link className="home-sidebar-item" to={hrefFor(item)}>
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
    <div className="home-layout">
      <aside className="home-sidebar">
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

      <div className="home-main-column">
        <h2 className="home-main-heading">Latest videos</h2>
        {videosError && (
          <p className="error">Failed to load recent videos: {videosError.message}</p>
        )}
        {!videosError && !videos && <p className="muted">Loading recent videos…</p>}
        {!videosError && videos && <VideoGrid videos={videos} />}
      </div>
    </div>
  )
}
