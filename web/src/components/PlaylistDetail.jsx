import { useState } from 'react'
import { usePolling } from '../usePolling'
import { fetchVideos, videoMediaUrl, deleteVideoFromCustomPlaylist } from '../api'
import { formatDateTime } from '../formatDateTime'
import { ConfirmDialog } from './ConfirmDialog'
import { PlaylistActionsMenu } from './PlaylistActionsMenu'

const STATUS_MESSAGES = {
  PENDING: 'This video is pending.',
  ERRORED_RETRYING: 'This video failed to download and will be retried.',
  ERRORED: 'This video failed to download.',
}

function VideoStatusIndicator({ status }) {
  if (status === 'DOWNLOADED') {
    return null
  }

  if (status === 'IN_PROGRESS') {
    return (
      <span className="video-status-icon video-status-icon-downloading" role="img" title="Downloading" aria-label="Downloading">
        ⬇
      </span>
    )
  }

  const message = STATUS_MESSAGES[status] ?? 'This video is pending.'
  return (
    <span className="video-status-icon video-status-icon-warning" role="img" title={message} aria-label={message}>
      ⚠
    </span>
  )
}

function VideoDetail({ playlist, video, onDeleted }) {
  const [confirmOpen, setConfirmOpen] = useState(false)
  const canDelete = playlist.kind === 'custom'

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
        <dd>{formatDateTime(video.created_at)}</dd>
        <dt>Updated at</dt>
        <dd>{formatDateTime(video.updated_at)}</dd>
      </dl>

      {canDelete && (
        <button className="danger-button video-detail-delete" onClick={() => setConfirmOpen(true)}>
          Delete
        </button>
      )}

      <ConfirmDialog
        open={confirmOpen}
        onOpenChange={setConfirmOpen}
        title={`Delete "${video.title}"?`}
        description="This removes the video's record and, if downloaded, its file."
        onConfirm={async () => {
          await deleteVideoFromCustomPlaylist(playlist.id, video.id)
          onDeleted?.()
        }}
      />
    </div>
  )
}

export function PlaylistDetail({ playlist, onBack, onDeleted }) {
  const { data: videos, error } = usePolling(
    () => fetchVideos(playlist.id),
    [playlist.id],
  )
  const [selectedVideo, setSelectedVideo] = useState(null)
  const [sidebarOpen, setSidebarOpen] = useState(false)

  const selectVideo = (video) => {
    setSelectedVideo(video)
    setSidebarOpen(false)
  }

  return (
    <div>
      <div className="playlist-detail-header">
        <button className="back-link" onClick={onBack}>
          ← Back to playlists
        </button>
        <div className="playlist-detail-title-row">
          <button
            className="secondary-button sidebar-toggle"
            onClick={() => setSidebarOpen((open) => !open)}
          >
            Videos
          </button>
          <h2>{playlist.name}</h2>
          <PlaylistActionsMenu playlist={playlist} onDeleted={onDeleted} />
        </div>
      </div>

      <div className="playlist-detail-layout">
        <div className={sidebarOpen ? 'video-sidebar open' : 'video-sidebar'}>
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
                    className={
                      selectedVideo?.id === video.id ? 'list-item active' : 'list-item'
                    }
                    onClick={() => selectVideo(video)}
                  >
                    <span className="list-item-title">{video.title}</span>
                    <VideoStatusIndicator status={video.status} />
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
        {sidebarOpen && (
          <div className="video-sidebar-backdrop" onClick={() => setSidebarOpen(false)} />
        )}

        <div className="video-player-pane">
          {selectedVideo?.status === 'DOWNLOADED' && selectedVideo.filename ? (
            // eslint-disable-next-line jsx-a11y/media-has-caption
            <video
              controls
              src={videoMediaUrl(playlist.path, selectedVideo.filename)}
            />
          ) : (
            <p className="muted">
              {selectedVideo
                ? 'This video has not been downloaded yet.'
                : 'Select a video to play it.'}
            </p>
          )}
        </div>

        <div className="video-detail-pane">
          {selectedVideo ? (
            <VideoDetail
              playlist={playlist}
              video={selectedVideo}
              onDeleted={() => setSelectedVideo(null)}
            />
          ) : (
            <p className="muted">No video selected.</p>
          )}
        </div>
      </div>
    </div>
  )
}
