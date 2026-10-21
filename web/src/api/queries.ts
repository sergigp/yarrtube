import { useCallback } from 'react'
import { useQuery, useQueryClient, type UseQueryResult } from '@tanstack/react-query'
import {
  fetchAnnouncements,
  fetchChannels,
  fetchChannelVideos,
  fetchHomeVideos,
  fetchPlaylists,
  fetchTasks,
  fetchVersion,
  fetchVideos,
  listCategories,
  listCategoryVideos,
  markCategoryWatched,
  markVideoWatched,
  previewChannel,
  previewPlaylist,
  updateChannel,
  updatePlaylist,
  type UpdateChannelRequest,
  type UpdatePlaylistRequest,
} from './client'
import type {
  Announcement,
  AppVersion,
  CategoryList,
  ChannelListItem,
  ChannelPreview,
  HomeVideo,
  HomeVideos,
  PlaylistListItem,
  PlaylistPreview,
  Task,
  Video,
} from './types'
import { anyWorkInProgress } from '@/lib/libraryPolling'

// The sidebar lists change rarely and are refetched right after the user's
// own actions; views showing live download progress poll quickly.
const LIBRARY_INTERVAL_MS = 60_000
const LIVE_INTERVAL_MS = 3_000
// Slower than the live views: each list reads every video and queued task.
const BUSY_SOURCES_INTERVAL_MS = 5_000

/**
 * The channel and playlist lists poll faster while a source has work in
 * progress, so its "Move…" enables within seconds of it settling.
 */
function sourcesInterval(items: { work_in_progress: boolean }[] | undefined): number {
  return anyWorkInProgress(items) ? BUSY_SOURCES_INTERVAL_MS : LIBRARY_INTERVAL_MS
}

export const queryKeys = {
  channels: ['channels'] as const,
  playlists: ['playlists'] as const,
  categories: ['categories'] as const,
  categoryVideos: (id: string) => ['categories', id, 'videos'] as const,
  channelVideos: (handle: string) => ['channels', handle, 'videos'] as const,
  playlistVideos: (id: string) => ['playlists', id, 'videos'] as const,
  recentVideos: ['videos', 'recent'] as const,
  tasks: ['tasks'] as const,
  announcements: ['announcements'] as const,
  version: ['version'] as const,
  // Outside the `['playlists']` and `['channels']` prefixes so refetching the
  // library leaves them be.
  playlistPreview: (value: string) => ['playlist-preview', value] as const,
  channelPreview: (value: string) => ['channel-preview', value] as const,
}

export function useChannels(): UseQueryResult<ChannelListItem[], Error> {
  return useQuery({
    queryKey: queryKeys.channels,
    queryFn: fetchChannels,
    refetchInterval: (query) => sourcesInterval(query.state.data),
  })
}

export function usePlaylists(): UseQueryResult<PlaylistListItem[], Error> {
  return useQuery({
    queryKey: queryKeys.playlists,
    queryFn: fetchPlaylists,
    refetchInterval: (query) => sourcesInterval(query.state.data),
  })
}

export function useCategories(): UseQueryResult<CategoryList, Error> {
  return useQuery({
    queryKey: queryKeys.categories,
    queryFn: listCategories,
    refetchInterval: LIBRARY_INTERVAL_MS,
  })
}

export function useCategoryVideos(id: string): UseQueryResult<HomeVideo[], Error> {
  return useQuery({
    queryKey: queryKeys.categoryVideos(id),
    queryFn: () => listCategoryVideos(id),
    refetchInterval: LIVE_INTERVAL_MS,
  })
}

/**
 * Looks the entered playlist ID or URL up on YouTube. Keyed by the value, so
 * a response never shows for input the field no longer holds.
 */
export function usePlaylistPreview(value: string): UseQueryResult<PlaylistPreview, Error> {
  return useQuery({
    queryKey: queryKeys.playlistPreview(value),
    queryFn: () => previewPlaylist(value),
    enabled: Boolean(value),
    retry: false,
    staleTime: Infinity,
  })
}

/**
 * Looks the entered channel handle or URL up on YouTube. Keyed by the value,
 * so a response never shows for input the field no longer holds.
 */
export function useChannelPreview(value: string): UseQueryResult<ChannelPreview, Error> {
  return useQuery({
    queryKey: queryKeys.channelPreview(value),
    queryFn: () => previewChannel(value),
    enabled: Boolean(value),
    retry: false,
    staleTime: Infinity,
  })
}

export function useChannelVideos(handle: string): UseQueryResult<Video[], Error> {
  return useQuery({
    queryKey: queryKeys.channelVideos(handle),
    queryFn: () => fetchChannelVideos(handle),
    refetchInterval: LIVE_INTERVAL_MS,
  })
}

export function usePlaylistVideos(id: string): UseQueryResult<Video[], Error> {
  return useQuery({
    queryKey: queryKeys.playlistVideos(id),
    queryFn: () => fetchVideos(id),
    refetchInterval: LIVE_INTERVAL_MS,
  })
}

export function useRecentVideos(): UseQueryResult<HomeVideos, Error> {
  return useQuery({
    queryKey: queryKeys.recentVideos,
    queryFn: fetchHomeVideos,
    refetchInterval: LIVE_INTERVAL_MS,
  })
}

export function useTasks(): UseQueryResult<Task[], Error> {
  return useQuery({
    queryKey: queryKeys.tasks,
    queryFn: fetchTasks,
    refetchInterval: LIVE_INTERVAL_MS,
  })
}

/** Fetched once per page load: announcements change rarely and a stale one is harmless. */
export function useAnnouncements(): UseQueryResult<Announcement[], Error> {
  return useQuery({
    queryKey: queryKeys.announcements,
    queryFn: fetchAnnouncements,
    staleTime: Infinity,
    refetchOnWindowFocus: false,
    retry: false,
  })
}

/** Fetched once per page load: the running version only changes on a restart. */
export function useVersion(): UseQueryResult<AppVersion, Error> {
  return useQuery({
    queryKey: queryKeys.version,
    queryFn: fetchVersion,
    staleTime: Infinity,
    refetchOnWindowFocus: false,
    retry: false,
  })
}

/**
 * Returns a function that refetches the channel, playlist and category lists
 * (and, by prefix, their video lists) after an action that changed them.
 */
export function useInvalidateLibrary(): () => Promise<unknown> {
  const queryClient = useQueryClient()
  return useCallback(
    () =>
      Promise.all([
        queryClient.invalidateQueries({ queryKey: queryKeys.channels }),
        queryClient.invalidateQueries({ queryKey: queryKeys.playlists }),
        queryClient.invalidateQueries({ queryKey: queryKeys.categories }),
      ]),
    [queryClient],
  )
}

/** Returns a function that refetches the task list, e.g. after a sync run on demand. */
export function useInvalidateTasks(): () => Promise<unknown> {
  const queryClient = useQueryClient()
  return useCallback(
    () => queryClient.invalidateQueries({ queryKey: queryKeys.tasks }),
    [queryClient],
  )
}

/**
 * Returns a function that wraps an async action so the channel and playlist
 * lists refetch once it succeeds.
 */
export function useLibraryAction() {
  const invalidateLibrary = useInvalidateLibrary()
  return useCallback(
    <Args extends unknown[]>(action: (...args: Args) => Promise<unknown>) =>
      async (...args: Args) => {
        await action(...args)
        invalidateLibrary()
      },
    [invalidateLibrary],
  )
}

/**
 * Returns a function that marks a video watched, then refetches the channel
 * and playlist lists (and their video lists) and the home videos.
 */
export function useMarkVideoWatched(): (youtubeId: string) => Promise<void> {
  const invalidateLibraryAndHome = useInvalidateLibraryAndHome()
  return useCallback(
    async (youtubeId: string) => {
      await markVideoWatched(youtubeId)
      await invalidateLibraryAndHome()
    },
    [invalidateLibraryAndHome],
  )
}

/**
 * Returns a function that marks every downloaded video of a category watched,
 * then refetches the library (category videos included) and the home videos.
 */
export function useMarkCategoryWatched(): (id: string) => Promise<void> {
  const invalidateLibraryAndHome = useInvalidateLibraryAndHome()
  return useCallback(
    async (id: string) => {
      await markCategoryWatched(id)
      await invalidateLibraryAndHome()
    },
    [invalidateLibraryAndHome],
  )
}

/**
 * Returns a function that sets whether a playlist is excluded from home, then
 * refetches the channel and playlist lists and the home videos.
 */
export function useSetPlaylistExcludedFromHome(): (id: string, excluded: boolean) => Promise<void> {
  const invalidateLibraryAndHome = useInvalidateLibraryAndHome()
  return useCallback(
    async (id: string, excluded: boolean) => {
      await updatePlaylist(id, { exclude_from_home: excluded })
      await invalidateLibraryAndHome()
    },
    [invalidateLibraryAndHome],
  )
}

/** Sets whether a playlist's unwatched count is hidden, then refetches the library. */
export function useSetPlaylistUnwatchedCountHidden(): (id: string, hidden: boolean) => Promise<void> {
  const invalidateLibrary = useInvalidateLibrary()
  return useCallback(
    async (id: string, hidden: boolean) => {
      await updatePlaylist(id, { hide_unwatched_count: hidden })
      await invalidateLibrary()
    },
    [invalidateLibrary],
  )
}

/**
 * Returns a function that changes a channel's settings, then refetches the
 * channel and playlist lists.
 */
export function useUpdateChannelSettings(): (
  handle: string,
  changes: UpdateChannelRequest,
) => Promise<void> {
  const invalidateLibrary = useInvalidateLibrary()
  return useCallback(
    async (handle: string, changes: UpdateChannelRequest) => {
      await updateChannel(handle, changes)
      await invalidateLibrary()
    },
    [invalidateLibrary],
  )
}

/**
 * Returns a function that changes a playlist's settings, then refetches the
 * channel and playlist lists.
 */
export function useUpdatePlaylistSettings(): (
  id: string,
  changes: UpdatePlaylistRequest,
) => Promise<void> {
  const invalidateLibrary = useInvalidateLibrary()
  return useCallback(
    async (id: string, changes: UpdatePlaylistRequest) => {
      await updatePlaylist(id, changes)
      await invalidateLibrary()
    },
    [invalidateLibrary],
  )
}

/** Refetches the channel and playlist lists (and their video lists) and the home videos. */
function useInvalidateLibraryAndHome(): () => Promise<unknown> {
  const queryClient = useQueryClient()
  const invalidateLibrary = useInvalidateLibrary()
  return useCallback(
    () =>
      Promise.all([
        invalidateLibrary(),
        queryClient.invalidateQueries({ queryKey: queryKeys.recentVideos }),
      ]),
    [queryClient, invalidateLibrary],
  )
}

/**
 * Returns a function that drops a cached query outright, e.g. a deleted
 * entry's video list, so refetching the library doesn't request it again.
 */
export function useRemoveQuery(): (queryKey: readonly unknown[]) => void {
  const queryClient = useQueryClient()
  return useCallback(
    (queryKey: readonly unknown[]) => {
      queryClient.removeQueries({ queryKey, exact: true })
    },
    [queryClient],
  )
}
