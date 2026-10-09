import type {
  Announcement,
  ChannelListItem,
  ChannelPreview,
  CreatedChannel,
  DirectoryListing,
  HomeVideos,
  PlaylistListItem,
  PlaylistPreview,
  RecordProgressResponse,
  Task,
  Video,
  VideoProgress,
} from './types'
import { parseAnnouncements } from '@/lib/announcements'

interface RequestOptions {
  method?: 'GET' | 'POST' | 'PATCH' | 'DELETE'
  body?: unknown
  /** Names the request in the fallback error message; defaults to the path. */
  label?: string
}

/**
 * Sends a request under `/api`; rejects with the server's error message when
 * it gives one. Returns the raw `Response`, leaving body parsing to the
 * typed wrappers below (DELETE and action endpoints respond with no body).
 */
async function send(path: string, { method = 'GET', body, label }: RequestOptions = {}): Promise<Response> {
  const response = await fetch(`/api${path}`, {
    method,
    ...(body !== undefined && {
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(body),
    }),
  })
  if (!response.ok) {
    const errorBody = (await response.json().catch(() => null)) as { error?: string } | null
    throw new Error(
      errorBody?.error ?? `request to ${label ?? path} failed with status ${response.status}`,
    )
  }
  return response
}

async function request<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const response = await send(path, options)
  return response.json() as Promise<T>
}

export function fetchPlaylists(): Promise<PlaylistListItem[]> {
  return request('/playlists')
}

export function fetchVideos(playlistId: string): Promise<Video[]> {
  return request(`/playlists/${encodeURIComponent(playlistId)}/videos`)
}

export function fetchHomeVideos(): Promise<HomeVideos> {
  return request('/videos/home')
}

export function fetchTasks(): Promise<Task[]> {
  return request('/tasks')
}

/**
 * Lists the immediate subdirectories of `path`, relative to the configured
 * videos root; an empty or omitted `path` lists the root itself. `root` in
 * the response is the absolute videos root the add dialog's destination
 * preview is built from.
 */
export function fetchDirectories(path?: string): Promise<DirectoryListing> {
  const query = path ? `?path=${encodeURIComponent(path)}` : ''
  return request(`/directories${query}`)
}

export function videoMediaUrl(playlistPath: string, filename: string): string {
  const encodedPath = playlistPath.split('/').map(encodeURIComponent).join('/')
  const encodedFilename = filename.split('/').map(encodeURIComponent).join('/')
  return `/media/${encodedPath}/${encodedFilename}`
}

export function avatarMediaUrl(filename: string): string {
  return `/avatars/${encodeURIComponent(filename)}`
}

export interface CreatePlaylistRequest {
  playlist: string
  path: string
  quality: string
  exclude_from_home: boolean
}

export function createPlaylist(body: CreatePlaylistRequest): Promise<PlaylistListItem> {
  return request('/playlists', { method: 'POST', body, label: '/playlists' })
}

/**
 * Looks a playlist ID or URL up on YouTube without tracking it; rejects with
 * the server's error message.
 */
export function previewPlaylist(playlist: string): Promise<PlaylistPreview> {
  return request(`/playlists/preview?playlist=${encodeURIComponent(playlist)}`)
}

export function previewChannel(channel: string): Promise<ChannelPreview> {
  return request(`/channels/preview?channel=${encodeURIComponent(channel)}`)
}

/** Changes whether a playlist's videos are left out of the home view. */
export function updatePlaylist(
  id: string,
  body: { exclude_from_home: boolean },
): Promise<PlaylistListItem> {
  return request(`/playlists/${encodeURIComponent(id)}`, {
    method: 'PATCH',
    body,
    label: `update playlist ${id}`,
  })
}

export async function deletePlaylist(id: string): Promise<void> {
  await send(`/playlists/${encodeURIComponent(id)}`, {
    method: 'DELETE',
    label: `delete playlist ${id}`,
  })
}

export async function reconcilePlaylist(id: string): Promise<void> {
  await send(`/playlists/${encodeURIComponent(id)}/reconcile`, {
    method: 'POST',
    label: `reconcile playlist ${id}`,
  })
}

export function fetchChannels(): Promise<ChannelListItem[]> {
  return request('/channels')
}

export function fetchChannelVideos(handle: string): Promise<Video[]> {
  return request(`/channels/${encodeURIComponent(handle)}/videos`)
}

export async function reconcileChannel(handle: string): Promise<void> {
  await send(`/channels/${encodeURIComponent(handle)}/reconcile`, {
    method: 'POST',
    label: `reconcile channel ${handle}`,
  })
}

export interface CreateChannelRequest {
  channel: string
  quality: string
  video_limit: number
  path: string
}

export function createChannel(body: CreateChannelRequest): Promise<CreatedChannel> {
  return request('/channels', { method: 'POST', body, label: '/channels' })
}

/** Only the settings to change; an absent one keeps its stored value. */
export interface UpdateChannelRequest {
  quality?: string
  video_limit?: number
}

/** Changes a channel's quality and/or video limit, applied from its next sync. */
export function updateChannel(handle: string, body: UpdateChannelRequest): Promise<CreatedChannel> {
  return request(`/channels/${encodeURIComponent(handle)}`, {
    method: 'PATCH',
    body,
    label: `update channel ${handle}`,
  })
}

export async function deleteChannel(id: string): Promise<void> {
  await send(`/channels/${encodeURIComponent(id)}`, {
    method: 'DELETE',
    label: `delete channel ${id}`,
  })
}

export async function markChannelWatched(handle: string): Promise<void> {
  await send(`/channels/${encodeURIComponent(handle)}/watched`, {
    method: 'POST',
    label: `mark channel ${handle} watched`,
  })
}

export async function markVideoWatched(youtubeId: string): Promise<void> {
  await send(`/videos/${encodeURIComponent(youtubeId)}/watched`, {
    method: 'POST',
    label: `mark video ${youtubeId} watched`,
  })
}

export function recordVideoProgress(
  youtubeId: string,
  progress: VideoProgress,
): Promise<RecordProgressResponse> {
  return request(`/videos/${encodeURIComponent(youtubeId)}/progress`, {
    method: 'POST',
    body: progress,
    label: `record progress of video ${youtubeId}`,
  })
}

/**
 * Records progress with `navigator.sendBeacon`, which the browser still
 * delivers while the page is being closed or hidden.
 */
export function beaconVideoProgress(youtubeId: string, progress: VideoProgress): void {
  const body = new Blob([JSON.stringify(progress)], { type: 'application/json' })
  navigator.sendBeacon(`/api/videos/${encodeURIComponent(youtubeId)}/progress`, body)
}

/** Published on the public repository's `main`, so it can be edited without a release. */
export const ANNOUNCEMENTS_URL =
  'https://raw.githubusercontent.com/sergigp/yarrtube/main/announcements/announcements.json'

export async function fetchAnnouncements(): Promise<Announcement[]> {
  const response = await fetch(ANNOUNCEMENTS_URL)
  if (!response.ok) {
    throw new Error(`announcements request failed with status ${response.status}`)
  }
  return parseAnnouncements(await response.json())
}
