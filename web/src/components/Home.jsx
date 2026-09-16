import { usePolling } from '../usePolling'
import { fetchRecentVideos } from '../api'

export function Home({ onSelect }) {
  const { data: videos, error } = usePolling(fetchRecentVideos, [])

  if (error) {
    return <p className="error">Failed to load recent videos: {error.message}</p>
  }

  if (!videos) {
    return <p className="muted">Loading recent videos…</p>
  }

  if (videos.length === 0) {
    return <p className="muted">No videos synced yet.</p>
  }

  return (
    <ul className="list">
      {videos.map((video) => (
        <li key={`${video.source.kind}:${video.source.id}:${video.id}`} className="list-row">
          <button className="list-item" onClick={() => onSelect(video.source, video.id)}>
            <span className="list-item-title">{video.title}</span>
          </button>
        </li>
      ))}
    </ul>
  )
}
