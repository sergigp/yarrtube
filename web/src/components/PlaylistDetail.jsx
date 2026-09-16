import { useState } from 'react'
import { usePolling } from '../usePolling'
import { fetchVideos, videoMediaUrl, deleteVideoFromCustomPlaylist } from '../api'
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
  const path = video.filename ? `${playlist.path}/${video.filename}` : playlist.path

  return (
    <div className="video-detail">
      <span className="chip-group">
        <span className={`status-badge status-badge-${video.status}`}>{video.status}</span>
        <span className="status-badge">{video.quality ?? '—'}</span>
      </span>
      <p className="video-detail-path">{path}</p>

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

export function PlaylistDetail({ playlist, onBack, onDeleted, initialVideoId }) {
  const { data: videos, error } = usePolling(
    () => fetchVideos(playlist.id),
    [playlist.id],
  )
  const [manualSelection, setManualSelection] = useState(null)
  const deepLinkedVideo = !manualSelection && initialVideoId
    ? (videos?.find((video) => video.id === initialVideoId) ?? null)
    : null
  const selectedVideo = manualSelection ?? deepLinkedVideo
  const autoplay = selectedVideo !== null && selectedVideo === deepLinkedVideo

  return (
    <div>
      <div className="playlist-detail-header">
        <button className="back-link" onClick={onBack}>
          ← Back to playlists
        </button>
        <div className="playlist-detail-title-row">
          <h2>{playlist.name}</h2>
          <PlaylistActionsMenu playlist={playlist} onDeleted={onDeleted} />
        </div>
      </div>

      <div className="playlist-detail-layout">
        <div className="video-main-column">
          {selectedVideo && <h3 className="video-title-heading">{selectedVideo.title}</h3>}

          <div className="video-player-pane">
            {selectedVideo?.status === 'DOWNLOADED' && selectedVideo.filename ? (
              // eslint-disable-next-line jsx-a11y/media-has-caption
              <video
                controls
                autoPlay={autoplay}
                src={videoMediaUrl(playlist.path, selectedVideo.filename)}
                poster={
                  selectedVideo.thumbnail_filename
                    ? videoMediaUrl(playlist.path, selectedVideo.thumbnail_filename)
                    : undefined
                }
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
                onDeleted={() => setManualSelection(null)}
              />
            ) : (
              <p className="muted">No video selected.</p>
            )}
          </div>
        </div>

        <div className="video-sidebar">
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
                    onClick={() => setManualSelection(video)}
                  >
                    <span className="list-item-title">{video.title}</span>
                    <VideoStatusIndicator status={video.status} />
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </div>
  )
}
