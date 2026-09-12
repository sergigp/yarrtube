import { useState } from 'react'
import { usePolling } from '../usePolling'
import { fetchVideos, videoMediaUrl } from '../api'

function VideoDetail({ playlist, video }) {
  return (
    <div className="video-detail">
      <dl>
        <dt>Title</dt>
        <dd>{video.title}</dd>
        <dt>Status</dt>
        <dd>{video.status}</dd>
        <dt>Quality</dt>
        <dd>{video.quality ?? '—'}</dd>
        <dt>Filename</dt>
        <dd>{video.filename ?? '—'}</dd>
        <dt>Playlist path</dt>
        <dd>{playlist.path}</dd>
        <dt>Created at</dt>
        <dd>{video.created_at}</dd>
        <dt>Updated at</dt>
        <dd>{video.updated_at}</dd>
      </dl>

      {video.status === 'DOWNLOADED' && video.filename && (
        // eslint-disable-next-line jsx-a11y/media-has-caption
        <video
          controls
          src={videoMediaUrl(playlist.path, video.filename)}
        />
      )}
    </div>
  )
}

export function PlaylistDetail({ playlist, onBack }) {
  const { data: videos, error } = usePolling(
    () => fetchVideos(playlist.id),
    [playlist.id],
  )
  const [selectedVideo, setSelectedVideo] = useState(null)

  return (
    <div>
      <button className="back-link" onClick={onBack}>
        ← Back to playlists
      </button>
      <h2>{playlist.name}</h2>

      {error && <p className="error">Failed to load videos: {error.message}</p>}
      {!error && !videos && <p className="muted">Loading videos…</p>}
      {!error && videos && videos.length === 0 && (
        <p className="muted">No videos recorded for this playlist yet.</p>
      )}
      {!error && videos && videos.length > 0 && (
        <ul className="list">
          {videos.map((video) => (
            <li key={video.id}>
              <button
                className="list-item"
                onClick={() => setSelectedVideo(video)}
              >
                <span className="list-item-title">{video.title}</span>
                <span className={`status-badge status-${video.status.toLowerCase()}`}>
                  {video.status}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}

      {selectedVideo && (
        <VideoDetail playlist={playlist} video={selectedVideo} />
      )}
    </div>
  )
}
