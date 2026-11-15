import { useState } from 'react'
import { Link, useNavigate, useParams, useSearchParams } from 'react-router-dom'
import {
  queryKeys,
  useCategories,
  useCategoriesShown,
  useCategorizationAvailable,
  useDeleteSource,
  useLibraryAction,
  usePlaylists,
  usePlaylistVideos,
  useSetPlaylistExcludedFromHome,
  useSetPlaylistUnwatchedCountHidden,
  useUpdatePlaylistSettings,
} from '@/api/queries'
import {
  reconcilePlaylist,
  deletePlaylist,
  deleteSourceAiVerdicts,
  markPlaylistWatched,
} from '@/api/client'
import type { PlaylistListItem, Video } from '@/api/types'
import { useRememberedChoice } from '@/hooks/useRememberedChoice'
import { useVideoSelection } from '@/hooks/useVideoSelection'
import { isMember } from '@/lib/categories'
import { formatRelativeTime } from '@/lib/formatDateTime'
import {
  WATCH_FILTERS,
  asHomeVideo,
  episodeLabel,
  matchesWatchFilter,
  nextToPlay,
  sortVideos,
  sourceSummary,
  watchFilterCounts,
  type VideoSort,
} from '@/lib/sourceVideos'
import { categoryPath, playlistPath, youtubePlaylistUrl } from '@/lib/sourcePath'
import { videoPathIn } from '@/lib/videoPath'
import { DetailHeader } from './DetailHeader'
import { EditPlaylistDialog } from './EditPlaylistDialog'
import { PlayLink } from './PlayLink'
import { VideoGrid } from './VideoGrid'
import { VideoListToolbar } from './VideoListToolbar'
import { VideoGridSkeleton } from './VideoSkeletons'
import { WatchView } from './WatchView'

const PLAYLIST_SORTS: readonly VideoSort[] = ['playlist', 'newest', 'oldest']

/**
 * A playlist's page (`/playlists/:id`). Browsing: its header with "Play
 * next" (the first video in playlist order not watched yet), the
 * watch-state chips and sort (remembered per playlist), and its videos as
 * cards numbered by episode. Once a video is picked (`?video=`), the
 * watching layout: player, video info and the playlist as the queue, the
 * name in the header leading back to browsing.
 */
export function PlaylistDetail() {
  const { id = '' } = useParams()
  const [searchParams] = useSearchParams()
  const watching = searchParams.has('video')
  const navigate = useNavigate()
  const { data: playlists, error: playlistsError } = usePlaylists()
  const refreshing = useLibraryAction()
  const deleteSource = useDeleteSource()
  const setPlaylistExcludedFromHome = useSetPlaylistExcludedFromHome()
  const setPlaylistUnwatchedCountHidden = useSetPlaylistUnwatchedCountHidden()
  const updatePlaylistSettings = useUpdatePlaylistSettings()
  const [editing, setEditing] = useState(false)
  const categoriesShown = useCategoriesShown()
  const categorizationAvailable = useCategorizationAvailable()
  const { data: categoryList } = useCategories({ enabled: categoriesShown })
  const playlist = playlists?.find((item) => item.id === id) ?? null

  const { data: videos, error } = usePlaylistVideos(id)
  const selection = useVideoSelection(videos)

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

  const next = videos ? nextToPlay(videos) : null
  const categories = categoriesShown
    ? (categoryList?.categories ?? []).filter((category) =>
        isMember(category, { kind: 'playlist', id }),
      )
    : []

  return (
    <div className={watching ? 'flex flex-col md:h-full md:min-h-[480px]' : 'flex min-w-0 flex-col'}>
      <DetailHeader
        name={playlist.name}
        badge={{ kind: 'playlist' }}
        compact={watching}
        nameHref={watching ? playlistPath(id) : undefined}
        nameLinkLabel="Back to the playlist page"
        summary={
          videos
            ? sourceSummary({
                kind: 'Playlist',
                videoCount: videos.length,
                unwatchedCount: playlist.unwatched_count,
                syncedAgo: playlist.last_synced_at
                  ? formatRelativeTime(playlist.last_synced_at)
                  : null,
              })
            : null
        }
        chips={
          !watching &&
          categories.map((category) => (
            <Link
              key={category.id}
              to={categoryPath(category.id)}
              className="flex items-center gap-1.5 rounded-full bg-secondary px-2 py-0.5 text-secondary-foreground no-underline hover:bg-accent"
            >
              <span
                aria-hidden="true"
                className="size-1.5 rounded-full"
                style={{ background: category.color }}
              />
              {category.name}
            </Link>
          ))
        }
        primaryAction={
          !watching &&
          next && (
            <PlayLink
              to={videoPathIn(playlistPath(id), next.id)}
              label={['Play next', episodeLabel(next.episode)].filter(Boolean).join(' · ')}
              title={next.title}
            />
          )
        }
        unwatchedCount={playlist.unwatched_count}
        onSync={refreshing(() => reconcilePlaylist(id))}
        onMarkWatched={refreshing(() => markPlaylistWatched(id))}
        excludedFromHome={playlist.exclude_from_home}
        onSetExcludedFromHome={(excluded) => setPlaylistExcludedFromHome(id, excluded)}
        unwatchedCountHidden={playlist.hide_unwatched_count}
        onSetUnwatchedCountHidden={(hidden) => setPlaylistUnwatchedCountHidden(id, hidden)}
        onEditRequest={() => setEditing(true)}
        youtubeLink={{ href: youtubePlaylistUrl(id), label: 'Open playlist on YouTube' }}
        onDelete={refreshing(async () => {
          await deleteSource(queryKeys.playlistVideos(id), () => deletePlaylist(id))
          navigate('/')
        })}
        deleteDescription="This removes the playlist from tracking, along with its video records and downloaded files."
        aiCategorization={
          categoriesShown
            ? {
                enabled: categorizationAvailable && playlist.categorization_enabled,
                onDeleteAiVerdicts: refreshing(() =>
                  deleteSourceAiVerdicts({ kind: 'playlist', id }),
                ),
              }
            : undefined
        }
      />
      <EditPlaylistDialog
        playlist={playlist}
        open={editing}
        onOpenChange={setEditing}
        onSave={(changes) => updatePlaylistSettings(id, changes)}
      />
      {watching ? (
        <WatchView
          noun="playlist"
          videos={videos}
          error={error}
          selection={selection}
          basePathOf={() => playlist.path}
          sourceNameOf={() => playlist.name}
          episodeOf={(video) => video.episode}
          orderLabel="playlist order"
        />
      ) : (
        <PlaylistBrowser playlist={playlist} videos={videos} error={error} />
      )}
    </div>
  )
}

/** The browsing body: chips and sort over the playlist's videos as cards. */
function PlaylistBrowser({
  playlist,
  videos,
  error,
}: {
  playlist: PlaylistListItem
  videos: Video[] | undefined
  error: Error | null
}) {
  const [filter, setFilter] = useRememberedChoice(
    `yarrtube.playlist.${playlist.id}.filter`,
    WATCH_FILTERS,
    'all',
  )
  const [sort, setSort] = useRememberedChoice(
    `yarrtube.playlist.${playlist.id}.sort`,
    PLAYLIST_SORTS,
    'playlist',
  )

  if (error) {
    return <p className="text-sm text-destructive">Failed to load videos: {error.message}</p>
  }
  if (!videos) {
    return <VideoGridSkeleton count={12} />
  }
  if (videos.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">No videos recorded for this playlist yet.</p>
    )
  }

  const episodes = new Map(videos.map((video) => [video.id, video.episode]))
  const source = {
    kind: 'playlist' as const,
    id: playlist.id,
    name: playlist.name,
    path: playlist.path,
    avatar_filename: null,
  }
  const shown = sortVideos(
    videos.filter((video) => matchesWatchFilter(video, filter)),
    sort,
  ).map((video) => asHomeVideo(video, source))

  return (
    <>
      <VideoListToolbar
        counts={watchFilterCounts(videos)}
        filter={filter}
        onFilterChange={setFilter}
        sorts={PLAYLIST_SORTS}
        sort={sort}
        onSortChange={setSort}
      />
      {shown.length === 0 ? (
        <p className="text-sm text-muted-foreground">No videos match this filter.</p>
      ) : (
        <VideoGrid
          videos={shown}
          layout="feed"
          videoPathOf={(video) => videoPathIn(playlistPath(playlist.id), video.id)}
          labelOf={(video) => episodeLabel(episodes.get(video.id))}
        />
      )}
    </>
  )
}
