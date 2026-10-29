import { useState } from 'react'
import { useChannels, useSourceVideos } from '@/api/queries'
import type { HomeVideo } from '@/api/types'
import { usePlaybackSpeed } from '@/hooks/usePlaybackSpeed'
import { useVideoSelection } from '@/hooks/useVideoSelection'
import { useWatchProgress } from '@/hooks/useWatchProgress'
import { VideoDetail } from './VideoDetail'
import { VideoListPane } from './VideoListPane'
import { VideoPlayer } from './VideoPlayer'

interface CategoryPlayerProps {
  videos: HomeVideo[] | undefined
  error: Error | null
}

/**
 * A category played like a playlist: the player and the selected video's
 * detail, with the category's videos as the queue beside them. Each video
 * plays from its own channel or playlist, whose video list supplies what the
 * player and detail need beyond the queue's entry.
 */
export function CategoryPlayer({ videos, error }: CategoryPlayerProps) {
  const { selectedVideo: selected, autoplay, selectVideo } = useVideoSelection(videos)
  const source = selected?.source ?? null
  const { data: sourceVideos } = useSourceVideos(source)
  const { data: channels } = useChannels()
  const video = sourceVideos?.find((candidate) => candidate.id === selected?.id) ?? null
  const channel =
    source?.kind === 'channel' ? channels?.find((item) => item.id === source.id) : undefined
  const [videoElement, setVideoElement] = useState<HTMLVideoElement | null>(null)
  useWatchProgress(videoElement, video)
  const playbackSpeed = usePlaybackSpeed(videoElement, video?.id ?? null)

  return (
    <div className="grid grid-cols-1 gap-4 md:min-h-0 md:flex-1 md:grid-cols-[minmax(0,1fr)_360px] md:items-start md:gap-6 md:overflow-hidden">
      <div className="contents md:flex md:h-full md:min-w-0 md:flex-col md:gap-4 md:overflow-y-auto">
        {source && (
          <VideoPlayer
            basePath={source.path}
            source={source}
            video={video}
            autoplay={autoplay}
            onVideoElement={setVideoElement}
          />
        )}

        {source && video ? (
          <VideoDetail
            basePath={source.path}
            video={video}
            channel={channel}
            playbackRate={playbackSpeed.rate}
            onPlaybackRateChange={playbackSpeed.changeRate}
          />
        ) : (
          <p className="text-sm text-muted-foreground">
            {selected ? 'Loading video…' : 'No video selected.'}
          </p>
        )}
      </div>

      <VideoListPane
        noun="category"
        basePathOf={(item) => item.source.path}
        videos={videos}
        error={error}
        selectedVideoId={selected?.id ?? null}
        onSelect={selectVideo}
      />
    </div>
  )
}
