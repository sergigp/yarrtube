import { useState } from 'react'
import type { Video } from '@/api/types'
import { usePlaybackSpeed } from '@/hooks/usePlaybackSpeed'
import type { VideoSelection } from '@/hooks/useVideoSelection'
import { useWatchProgress } from '@/hooks/useWatchProgress'
import { episodeLabel } from '@/lib/sourceVideos'
import { VideoDetail, type VideoDetailSource } from './VideoDetail'
import { VideoListPane } from './VideoListPane'
import { VideoPlayer } from './VideoPlayer'

interface WatchViewProps<T extends Video> {
  /** Names the source in the queue's empty message ("channel", "playlist" or "category"). */
  noun: string
  videos: T[] | undefined
  error: Error | null
  selection: VideoSelection<T>
  /** The channel/playlist path a video is served under. */
  basePathOf: (video: T) => string
  /** The name of the channel or playlist a video comes from. */
  sourceNameOf: (video: T) => string
  /** The source the video info links, if any. */
  detailSourceOf?: (video: T) => VideoDetailSource | undefined
  /** A playlist's episode number per video. */
  episodeOf?: (video: T) => number | null
  /** How the queue is ordered, e.g. "playlist order". */
  orderLabel?: string
}

/**
 * The watching layout of a channel, playlist or category: the player and
 * the selected video's info, with the videos as the queue beside them
 * (below them on phones). Each video plays from, and records its progress
 * against, its own channel or playlist.
 */
export function WatchView<T extends Video>({
  noun,
  videos,
  error,
  selection,
  basePathOf,
  sourceNameOf,
  detailSourceOf,
  episodeOf,
  orderLabel,
}: WatchViewProps<T>) {
  const { selectedVideo: video, autoplay, selectVideo } = selection
  const [videoElement, setVideoElement] = useState<HTMLVideoElement | null>(null)
  useWatchProgress(videoElement, video)
  const playbackSpeed = usePlaybackSpeed(videoElement, video?.id ?? null)
  const episode = (video && episodeOf?.(video)) ?? null

  return (
    <div className="grid grid-cols-1 gap-4 md:min-h-0 md:flex-1 md:grid-cols-[minmax(0,1fr)_minmax(20rem,24rem)] md:items-start md:gap-6 md:overflow-hidden">
      <div className="contents md:flex md:h-full md:min-w-0 md:flex-col md:gap-4 md:overflow-y-auto">
        <VideoPlayer
          basePath={video ? basePathOf(video) : ''}
          video={video}
          autoplay={autoplay}
          onVideoElement={setVideoElement}
        />

        {video ? (
          <VideoDetail
            basePath={basePathOf(video)}
            video={video}
            source={detailSourceOf?.(video)}
            episodeLabel={episodeLabel(episode, 'long')}
            playbackRate={playbackSpeed.rate}
            onPlaybackRateChange={playbackSpeed.changeRate}
            videoElement={videoElement}
          />
        ) : (
          <p className="text-sm text-muted-foreground">No video selected.</p>
        )}
      </div>

      <VideoListPane
        noun={noun}
        basePathOf={basePathOf}
        sourceNameOf={sourceNameOf}
        videos={videos}
        error={error}
        selectedVideoId={video?.id ?? null}
        onSelect={selectVideo}
        episodeOf={episodeOf}
        orderLabel={orderLabel}
      />
    </div>
  )
}
