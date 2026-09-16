import { useState } from 'react'
import { useNavigate, useParams, useSearchParams } from 'react-router-dom'
import { usePolling } from '../usePolling'
import { fetchChannels, fetchChannelVideos, videoMediaUrl } from '../api'
import { ChannelActionsMenu } from './ChannelActionsMenu'

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

function VideoDetail({ channel, video }) {
  const path = video.filename ? `${channel.path}/${video.filename}` : channel.path

  return (
    <div className="video-detail">
      <span className="chip-group">
        <span className={`status-badge status-badge-${video.status}`}>{video.status}</span>
        <span className="status-badge">{video.quality ?? '—'}</span>
      </span>
      <p className="video-detail-path">{path}</p>
      <a
        className="video-detail-youtube-link"
        href={`https://www.youtube.com/watch?v=${encodeURIComponent(video.id)}`}
        target="_blank"
        rel="noopener"
      >
        Open on YouTube
      </a>
    </div>
  )
}

export function ChannelDetail() {
  const { id } = useParams()
  const [searchParams] = useSearchParams()
  const navigate = useNavigate()
  const { data: channels, error: channelsError } = usePolling(fetchChannels, [])
  const channel = channels?.find((item) => item.id === id) ?? null

  const { data: videos, error } = usePolling(() => fetchChannelVideos(id), [id])
  const [manualSelection, setManualSelection] = useState(null)
  const initialVideoId = searchParams.get('video')
  const deepLinkedVideo = !manualSelection && initialVideoId
    ? (videos?.find((video) => video.id === initialVideoId) ?? null)
    : null
  const defaultVideo = !manualSelection && !deepLinkedVideo ? (videos?.[0] ?? null) : null
  const selectedVideo = manualSelection ?? deepLinkedVideo ?? defaultVideo
  const autoplay = selectedVideo !== null && selectedVideo === deepLinkedVideo

  if (channelsError) {
    return <p className="error">Failed to load channel: {channelsError.message}</p>
  }

  if (!channels) {
    return <p className="muted">Loading channel…</p>
  }

  if (!channel) {
    return <p className="error">Channel not found.</p>
  }

  return (
    <div className="playlist-detail-route">
      <div className="playlist-detail-header">
        <div className="playlist-detail-title-row">
          <h2>{channel.name}</h2>
          <ChannelActionsMenu channel={channel} onDeleted={() => navigate('/')} />
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
                src={videoMediaUrl(channel.path, selectedVideo.filename)}
                poster={
                  selectedVideo.thumbnail_filename
                    ? videoMediaUrl(channel.path, selectedVideo.thumbnail_filename)
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
              <VideoDetail channel={channel} video={selectedVideo} />
            ) : (
              <p className="muted">No video selected.</p>
            )}
          </div>
        </div>

        <div className="video-sidebar">
          {error && <p className="error">Failed to load videos: {error.message}</p>}
          {!error && !videos && <p className="muted">Loading videos…</p>}
          {!error && videos && videos.length === 0 && (
            <p className="muted">No videos recorded for this channel yet.</p>
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
                    <span className="list-item-main">
                      {video.thumbnail_filename ? (
                        <img
                          className="list-item-thumbnail"
                          src={videoMediaUrl(channel.path, video.thumbnail_filename)}
                          alt=""
                        />
                      ) : (
                        <span className="list-item-thumbnail-placeholder" />
                      )}
                      <span className="list-item-title">{video.title}</span>
                    </span>
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
