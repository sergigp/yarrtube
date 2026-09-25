import { useState } from 'react'
import { useParams, useSearchParams } from 'react-router-dom'
import { CheckCheck, TriangleAlert } from 'lucide-react'
import { usePolling } from '../usePolling'
import {
  fetchChannels,
  fetchChannelVideos,
  markChannelWatched,
  videoMediaUrl,
  avatarMediaUrl,
} from '../api'
import { formatDuration } from '../formatDuration'
import { useWatchProgress } from '../useWatchProgress'
import { Thumbnail } from './Thumbnail'
import { WatchedTick } from './WatchedTick'
import { Beacon } from './Beacon'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'

const STATUS_MESSAGES = {
  PENDING: 'This video is pending.',
  ERRORED_RETRYING: 'This video failed to download and will be retried.',
  ERRORED: 'This video failed to download.',
}

const STATUS_LABELS = {
  DOWNLOADED: 'Downloaded',
  IN_PROGRESS: 'Downloading',
  PENDING: 'Pending',
  ERRORED_RETRYING: 'Retrying',
  ERRORED: 'Errored',
}

const QUALITY_LABELS = {
  high: 'High quality',
  mid: 'Medium quality',
  low: 'Low quality',
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

function VideoDetail({ channel, video }) {
  const path = video.filename ? `${channel.path}/${video.filename}` : channel.path

  return (
    <div className="rounded-lg border border-border p-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="flex min-w-0 flex-1 items-start gap-2.5">
          <Thumbnail
            src={channel.avatar_filename ? avatarMediaUrl(channel.avatar_filename) : null}
            className="mt-0.5 size-8 shrink-0 rounded-full object-cover"
          />
          <h3 className="min-w-0 flex-1 font-heading text-xl font-semibold text-foreground">
            {video.title}
          </h3>
        </div>
        <div className="flex shrink-0 items-center gap-1.5">
          <Badge variant={video.status === 'DOWNLOADED' ? 'secondary' : 'outline'}>
            {STATUS_LABELS[video.status] ?? video.status}
          </Badge>
          <Badge variant="outline">{QUALITY_LABELS[video.quality] ?? '—'}</Badge>
        </div>
      </div>
      <p className="mt-2 text-xs break-words text-muted-foreground">{path}</p>
      <a
        className="mt-3 inline-block text-sm text-primary underline-offset-4 hover:underline"
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
  const { data: channels, error: channelsError } = usePolling(fetchChannels, [])
  const channel = channels?.find((item) => item.id === id) ?? null

  const { data: videos, error } = usePolling(() => fetchChannelVideos(id), [id])
  const [manualSelectionId, setManualSelectionId] = useState(null)
  const [markingWatched, setMarkingWatched] = useState(false)
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
    <div className="flex h-full min-h-[480px] flex-col">
      <div className="grid min-h-0 flex-1 grid-cols-1 items-start gap-6 overflow-hidden md:grid-cols-[minmax(0,1fr)_360px]">
        <div className="flex min-w-0 flex-col gap-4 h-full overflow-hidden overflow-y-auto">
          <div className="flex min-h-80 items-center justify-center rounded-lg bg-secondary/60">
            {selectedVideo?.status === 'DOWNLOADED' && selectedVideo.filename ? (
              // eslint-disable-next-line jsx-a11y/media-has-caption
              <video
                ref={setVideoElement}
                controls
                autoPlay={autoplay}
                className="block max-h-[70vh] w-full rounded-lg"
                src={videoMediaUrl(channel.path, selectedVideo.filename)}
                poster={
                  selectedVideo.thumbnail_filename
                    ? videoMediaUrl(channel.path, selectedVideo.thumbnail_filename)
                    : undefined
                }
              />
            ) : (
              <p className="text-sm text-muted-foreground">
                {selectedVideo
                  ? 'This video has not been downloaded yet.'
                  : 'Select a video to play it.'}
              </p>
            )}
          </div>

          {selectedVideo ? (
            <VideoDetail channel={channel} video={selectedVideo} />
          ) : (
            <p className="text-sm text-muted-foreground">No video selected.</p>
          )}
        </div>

        <div className="min-h-0 h-full overflow-y-auto">
          <div className="mb-2 flex justify-end">
            <Button
              variant="outline"
              size="sm"
              disabled={markingWatched}
              onClick={async () => {
                setMarkingWatched(true)
                try {
                  await markChannelWatched(id)
                } catch (err) {
                  window.alert(`Failed to mark "${channel.name}" watched: ${err.message}`)
                } finally {
                  setMarkingWatched(false)
                }
              }}
            >
              <CheckCheck />
              Mark all watched
            </Button>
          </div>
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
