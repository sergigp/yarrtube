import { useState } from 'react'
import { useNavigate, useParams, useSearchParams } from 'react-router-dom'
import { TriangleAlert } from 'lucide-react'
import { useChannels, useChannelVideos, useInvalidateLibrary } from '../queries'
import {
  reconcileChannel,
  deleteChannel,
  markChannelWatched,
  videoMediaUrl,
  avatarMediaUrl,
} from '../api'
import { formatDuration } from '../formatDuration'
import { useWatchProgress } from '../useWatchProgress'
import { Thumbnail } from './Thumbnail'
import { WatchedTick } from './WatchedTick'
import { Beacon } from './Beacon'
import { VideoPlayer } from './VideoPlayer'
import { VideoDetail } from './VideoDetail'
import { DetailHeader } from './DetailHeader'
import { cn } from '@/lib/utils'

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
    return <Beacon variant="live" label="Downloading" className="shrink-0" />
  }

  const message = STATUS_MESSAGES[status] ?? 'This video is pending.'
  return (
    <TriangleAlert
      className="size-3.5 shrink-0 text-amber-600"
      role="img"
      aria-label={message}
    >
      <title>{message}</title>
    </TriangleAlert>
  )
}

export function ChannelDetail() {
  const { id } = useParams()
  const navigate = useNavigate()
  const [searchParams] = useSearchParams()
  const { data: channels, error: channelsError } = useChannels()
  const invalidateLibrary = useInvalidateLibrary()
  const channel = channels?.find((item) => item.id === id) ?? null

  const { data: videos, error } = useChannelVideos(id)
  const [manualSelectionId, setManualSelectionId] = useState(null)
  const manualSelection = manualSelectionId
    ? (videos?.find((video) => video.id === manualSelectionId) ?? null)
    : null
  const initialVideoId = searchParams.get('video')
  const deepLinkedVideo = !manualSelection && initialVideoId
    ? (videos?.find((video) => video.id === initialVideoId) ?? null)
    : null
  const defaultVideo = !manualSelection && !deepLinkedVideo ? (videos?.[0] ?? null) : null
  const selectedVideo = manualSelection ?? deepLinkedVideo ?? defaultVideo
  const autoplay = selectedVideo !== null && selectedVideo === deepLinkedVideo
  const [videoElement, setVideoElement] = useState(null)
  useWatchProgress(videoElement, selectedVideo)

  if (channelsError) {
    return <p className="text-sm text-destructive">Failed to load channel: {channelsError.message}</p>
  }

  if (!channels) {
    return <p className="text-sm text-muted-foreground">Loading channel…</p>
  }

  if (!channel) {
    return <p className="text-sm text-destructive">Channel not found.</p>
  }

  return (
    <div className="flex flex-col md:h-full md:min-h-[480px]">
      <DetailHeader
        name={channel.name}
        showAvatar
        avatarSrc={channel.avatar_filename ? avatarMediaUrl(channel.avatar_filename) : null}
        videos={videos}
        unwatchedCount={channel.unwatched_count}
        onSync={async () => {
          await reconcileChannel(id)
          invalidateLibrary()
        }}
        onMarkWatched={async () => {
          await markChannelWatched(id)
          invalidateLibrary()
        }}
        onDelete={async () => {
          await deleteChannel(id)
          invalidateLibrary()
          navigate('/')
        }}
        deleteDescription="This removes the channel from tracking."
      />
      <div className="grid grid-cols-1 gap-4 md:min-h-0 md:flex-1 md:grid-cols-[minmax(0,1fr)_360px] md:items-start md:gap-6 md:overflow-hidden">
        <div className="contents md:flex md:h-full md:min-w-0 md:flex-col md:gap-4 md:overflow-y-auto">
          <VideoPlayer
            basePath={channel.path}
            video={selectedVideo}
            autoplay={autoplay}
            onVideoElement={setVideoElement}
          />

          {selectedVideo ? (
            <VideoDetail basePath={channel.path} video={selectedVideo} channel={channel} />
          ) : (
            <p className="text-sm text-muted-foreground">No video selected.</p>
          )}
        </div>

        <div className="md:h-full md:min-h-0 md:overflow-y-auto">
          {error && <p className="text-sm text-destructive">Failed to load videos: {error.message}</p>}
          {!error && !videos && <p className="text-sm text-muted-foreground">Loading videos…</p>}
          {!error && videos && videos.length === 0 && (
            <p className="text-sm text-muted-foreground">No videos recorded for this channel yet.</p>
          )}
          {!error && videos && videos.length > 0 && (
            <ul className="flex flex-col divide-y divide-border">
              {videos.map((video) => {
                const active = selectedVideo?.id === video.id
                return (
                  <li key={video.id}>
                    <button
                      className={cn(
                        'flex w-full items-start gap-3 rounded-md px-2 py-2.5 text-left transition-colors hover:bg-accent',
                        active && 'bg-accent',
                      )}
                      onClick={() => setManualSelectionId(video.id)}
                    >
                      <div className="relative shrink-0">
                        <Thumbnail
                          src={
                            video.thumbnail_filename
                              ? videoMediaUrl(channel.path, video.thumbnail_filename)
                              : null
                          }
                          className="aspect-video w-24 rounded-md object-cover"
                        />
                        <WatchedTick watched={video.watched} />
                        {formatDuration(video.duration_seconds) && (
                          <span className="absolute right-1 bottom-1 rounded bg-black/75 px-1 py-0.5 text-[10px] font-medium text-white">
                            {formatDuration(video.duration_seconds)}
                          </span>
                        )}
                      </div>
                      <span
                        className={cn(
                          'min-w-0 flex-1 text-sm leading-snug text-foreground',
                          active && 'font-medium text-primary',
                        )}
                      >
                        {video.title}
                      </span>
                      <VideoStatusIndicator status={video.status} />
                    </button>
                  </li>
                )
              })}
            </ul>
          )}
        </div>
      </div>
    </div>
  )
}
