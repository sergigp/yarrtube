import { useState } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import {
  queryKeys,
  useLibraryAction,
  usePlaylists,
  usePlaylistVideos,
  useRemoveQuery,
  useSetPlaylistExcludedFromHome,
} from '@/api/queries'
import { reconcilePlaylist, deletePlaylist } from '@/api/client'
import { useWatchProgress } from '@/hooks/useWatchProgress'
import { useVideoSelection } from '@/hooks/useVideoSelection'
import { VideoPlayer } from './VideoPlayer'
import { VideoDetail } from './VideoDetail'
import { VideoListPane } from './VideoListPane'
import { DetailHeader } from './DetailHeader'

export function PlaylistDetail() {
  const { id = '' } = useParams()
  const navigate = useNavigate()
  const { data: playlists, error: playlistsError } = usePlaylists()
  const refreshing = useLibraryAction()
  const removeQuery = useRemoveQuery()
  const setPlaylistExcludedFromHome = useSetPlaylistExcludedFromHome()
  const playlist = playlists?.find((item) => item.id === id) ?? null

  const { data: videos, error } = usePlaylistVideos(id)
  const { selectedVideo, autoplay, selectVideo } = useVideoSelection(videos)
  const [videoElement, setVideoElement] = useState<HTMLVideoElement | null>(null)
  useWatchProgress(videoElement, selectedVideo)

  if (playlistsError) {
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
        onSync={refreshing(() => reconcilePlaylist(id))}
        excludedFromHome={playlist.exclude_from_home}
        onSetExcludedFromHome={(excluded) => setPlaylistExcludedFromHome(id, excluded)}
        onDelete={refreshing(async () => {
          await deletePlaylist(id)
          removeQuery(queryKeys.playlistVideos(id))
          navigate('/')
        })}
        deleteDescription="This removes the playlist from tracking, along with its video records and downloaded files."
      />
      <div className="grid grid-cols-1 gap-4 md:min-h-0 md:flex-1 md:grid-cols-[minmax(0,1fr)_360px] md:items-start md:gap-6 md:overflow-hidden">
        <div className="contents md:flex md:h-full md:min-w-0 md:flex-col md:gap-4 md:overflow-y-auto">
          <VideoPlayer
            basePath={playlist.path}
            video={selectedVideo}
            autoplay={autoplay}
            onVideoElement={setVideoElement}
          />

          {selectedVideo ? (
            <VideoDetail basePath={playlist.path} video={selectedVideo} />
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
