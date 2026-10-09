import { useState } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import {
  queryKeys,
  useChannels,
  useChannelVideos,
  useLibraryAction,
  useRemoveQuery,
} from '@/api/queries'
import { reconcileChannel, deleteChannel, markChannelWatched, avatarMediaUrl } from '@/api/client'
import { useWatchProgress } from '@/hooks/useWatchProgress'
import { usePlaybackSpeed } from '@/hooks/usePlaybackSpeed'
import { useSaveChannelSettings } from '@/hooks/useSaveChannelSettings'
import { useVideoSelection } from '@/hooks/useVideoSelection'
import { VideoPlayer } from './VideoPlayer'
import { VideoDetail } from './VideoDetail'
import { VideoListPane } from './VideoListPane'
import { DetailHeader } from './DetailHeader'
import { EditChannelDialog } from './EditChannelDialog'

export function ChannelDetail() {
  const { id = '' } = useParams()
  const navigate = useNavigate()
  const { data: channels, error: channelsError } = useChannels()
  const refreshing = useLibraryAction()
  const removeQuery = useRemoveQuery()
  const saveChannelSettings = useSaveChannelSettings()
  const [editing, setEditing] = useState(false)
  const channel = channels?.find((item) => item.id === id) ?? null

  const { data: videos, error } = useChannelVideos(id)
  const { selectedVideo, autoplay, selectVideo } = useVideoSelection(videos)
  const [videoElement, setVideoElement] = useState<HTMLVideoElement | null>(null)
  useWatchProgress(videoElement, selectedVideo)
  const playbackSpeed = usePlaybackSpeed(videoElement, selectedVideo?.id ?? null)

  if (channelsError) {
    return (
      <p className="text-sm text-destructive">Failed to load channel: {channelsError.message}</p>
    )
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
        onSync={refreshing(() => reconcileChannel(id))}
        onMarkWatched={refreshing(() => markChannelWatched(id))}
        onEditRequest={() => setEditing(true)}
        onDelete={refreshing(async () => {
          await deleteChannel(id)
          removeQuery(queryKeys.channelVideos(id))
          navigate('/')
        })}
        deleteDescription="This removes the channel from tracking."
      />
      <EditChannelDialog
        channel={channel}
        open={editing}
        onOpenChange={setEditing}
        onSave={(changes) => saveChannelSettings(channel, changes)}
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
            <VideoDetail
              basePath={channel.path}
              video={selectedVideo}
              channel={channel}
              playbackRate={playbackSpeed.rate}
              onPlaybackRateChange={playbackSpeed.changeRate}
            />
          ) : (
            <p className="text-sm text-muted-foreground">No video selected.</p>
          )}
        </div>

        <VideoListPane
          noun="channel"
          basePath={channel.path}
          videos={videos}
          error={error}
          selectedVideoId={selectedVideo?.id ?? null}
          onSelect={selectVideo}
        />
      </div>
    </div>
  )
}
