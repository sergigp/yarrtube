import { useCallback } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import {
  fetchChannels,
  fetchChannelVideos,
  fetchHomeVideos,
  fetchPlaylists,
  fetchTasks,
  fetchVideos,
} from './api'

// The sidebar lists change rarely and are refetched right after the user's
// own actions; views showing live download progress poll quickly.
const LIBRARY_INTERVAL_MS = 60_000
const LIVE_INTERVAL_MS = 3_000

export const queryKeys = {
  channels: ['channels'],
  playlists: ['playlists'],
  channelVideos: (handle) => ['channels', handle, 'videos'],
  playlistVideos: (id) => ['playlists', id, 'videos'],
  recentVideos: ['videos', 'recent'],
  tasks: ['tasks'],
}

export function useChannels() {
  return useQuery({
    queryKey: queryKeys.channels,
    queryFn: fetchChannels,
    refetchInterval: LIBRARY_INTERVAL_MS,
  })
}

export function usePlaylists() {
  return useQuery({
    queryKey: queryKeys.playlists,
    queryFn: fetchPlaylists,
    refetchInterval: LIBRARY_INTERVAL_MS,
  })
}

export function useChannelVideos(handle) {
  return useQuery({
    queryKey: queryKeys.channelVideos(handle),
    queryFn: () => fetchChannelVideos(handle),
    refetchInterval: LIVE_INTERVAL_MS,
  })
}

export function usePlaylistVideos(id) {
  return useQuery({
    queryKey: queryKeys.playlistVideos(id),
    queryFn: () => fetchVideos(id),
    refetchInterval: LIVE_INTERVAL_MS,
  })
}

export function useRecentVideos() {
  return useQuery({
    queryKey: queryKeys.recentVideos,
    queryFn: fetchHomeVideos,
    refetchInterval: LIVE_INTERVAL_MS,
  })
}

export function useTasks() {
  return useQuery({
    queryKey: queryKeys.tasks,
    queryFn: fetchTasks,
    refetchInterval: LIVE_INTERVAL_MS,
  })
}

/**
 * Returns a function that refetches the channel and playlist lists (and,
 * by prefix, their video lists) after an action that changed them.
 */
export function useInvalidateLibrary() {
  const queryClient = useQueryClient()
  return useCallback(
    () =>
      Promise.all([
        queryClient.invalidateQueries({ queryKey: queryKeys.channels }),
        queryClient.invalidateQueries({ queryKey: queryKeys.playlists }),
      ]),
    [queryClient],
  )
}
