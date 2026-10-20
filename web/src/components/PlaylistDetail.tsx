import { useState } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import {
  queryKeys,
  useLibraryAction,
  usePlaylists,
  usePlaylistVideos,
  useRemoveQuery,
  useSetPlaylistExcludedFromHome,
  useSetPlaylistUnwatchedCountHidden,
  useUpdatePlaylistSettings,
} from '@/api/queries'
import { reconcilePlaylist, deletePlaylist, markPlaylistWatched } from '@/api/client'
import { useWatchProgress } from '@/hooks/useWatchProgress'
import { usePlaybackSpeed } from '@/hooks/usePlaybackSpeed'
import { useVideoSelection } from '@/hooks/useVideoSelection'
import { VideoPlayer } from './VideoPlayer'
import { VideoDetail } from './VideoDetail'
import { VideoListPane } from './VideoListPane'
import { DetailHeader } from './DetailHeader'
import { EditPlaylistDialog } from './EditPlaylistDialog'

export function PlaylistDetail() {
  const { id = '' } = useParams()
  const navigate = useNavigate()
  const { data: playlists, error: playlistsError } = usePlaylists()
  const refreshing = useLibraryAction()
  const removeQuery = useRemoveQuery()
  const setPlaylistExcludedFromHome = useSetPlaylistExcludedFromHome()
  const setPlaylistUnwatchedCountHidden = useSetPlaylistUnwatchedCountHidden()
  const updatePlaylistSettings = useUpdatePlaylistSettings()
  const [editing, setEditing] = useState(false)
  const playlist = playlists?.find((item) => item.id === id) ?? null

  const { data: videos, error } = usePlaylistVideos(id)
  const { selectedVideo, autoplay, selectVideo } = useVideoSelection(videos)
  const [videoElement, setVideoElement] = useState<HTMLVideoElement | null>(null)
  useWatchProgress(videoElement, selectedVideo)
  const playbackSpeed = usePlaybackSpeed(videoElement, selectedVideo?.id ?? null)

  // A failed background refresh keeps the loaded playlist, and its player, on screen.
  if (playlistsError && !playlists) {
    return (
      <p className="text-sm text-destructive">Failed to load playlist: {playlistsError.message}</p>
    )
  }

  if (!playlists) {
    return <p className="text-sm text-muted-foreground">Loading playlist…</p>
  }

  if (!playlist) {
    return <p className="text-sm text-destructive">Playlist not found.</p>
  }

  return (
    <div className="flex flex-col md:h-full md:min-h-[480px]">
      <DetailHeader
        name={playlist.name}
        videos={videos}
        unwatchedCount={playlist.unwatched_count}
        onSync={refreshing(() => reconcilePlaylist(id))}
        onMarkWatched={refreshing(() => markPlaylistWatched(id))}
        excludedFromHome={playlist.exclude_from_home}
        onSetExcludedFromHome={(excluded) => setPlaylistExcludedFromHome(id, excluded)}
        unwatchedCountHidden={playlist.hide_unwatched_count}
        onSetUnwatchedCountHidden={(hidden) => setPlaylistUnwatchedCountHidden(id, hidden)}
        onEditRequest={() => setEditing(true)}
        onDelete={refreshing(async () => {
          await deletePlaylist(id)
          removeQuery(queryKeys.playlistVideos(id))
          navigate('/')
        })}
        deleteDescription="This removes the playlist from tracking, along with its video records and downloaded files."
      />
      <EditPlaylistDialog
        playlist={playlist}
        open={editing}
        onOpenChange={setEditing}
        onSave={(changes) => updatePlaylistSettings(id, changes)}
      />
      <div className="grid grid-cols-1 gap-4 md:min-h-0 md:flex-1 md:grid-cols-[minmax(0,1fr)_360px] md:items-start md:gap-6 md:overflow-hidden">
        <div className="contents md:flex md:h-full md:min-w-0 md:flex-col md:gap-4 md:overflow-y-auto">
          <VideoPlayer
            basePath={playlist.path}
            source={{ kind: 'playlist', id }}
            video={selectedVideo}
            autoplay={autoplay}
            onVideoElement={setVideoElement}
          />

          {selectedVideo ? (
            <VideoDetail
              basePath={playlist.path}
              video={selectedVideo}
              playbackRate={playbackSpeed.rate}
              onPlaybackRateChange={playbackSpeed.changeRate}
            />
          ) : (
            <p className="text-sm text-muted-foreground">No video selected.</p>
          )}
        </div>

        <VideoListPane
          noun="playlist"
          basePath={playlist.path}
          videos={videos}
          error={error}
          selectedVideoId={selectedVideo?.id ?? null}
          onSelect={selectVideo}
        />
      </div>
    </div>
  )
}
