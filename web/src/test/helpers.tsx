import type { ReactElement, ReactNode } from 'react'
import { render } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter } from 'react-router-dom'
import { vi } from 'vitest'
import type {
  Announcement,
  ChannelListItem,
  HomeVideo,
  PlaylistListItem,
  Task,
  Video,
} from '@/api/types'

/**
 * Stubs `fetch` with a route table: keys are `"METHOD /api/path"` (query
 * string included), values the JSON to respond with, an HTTP error, or a
 * function computing either per call (a returned promise is awaited, so a
 * never-resolving one keeps the request pending). Unrouted requests reject,
 * so a test only ever exercises the endpoints it declared.
 */
export type RouteResponse = unknown | { status: number; error: string }
export type Routes = Record<string, RouteResponse | (() => RouteResponse | Promise<RouteResponse>)>

export function mockApi(routes: Routes): ReturnType<typeof vi.fn> {
  const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url
    const method = init?.method ?? 'GET'
    const key = `${method} ${url}`
    if (!(key in routes)) {
      throw new Error(`unrouted request: ${key}`)
    }
    const route = routes[key]
    const value = await (typeof route === 'function'
      ? (route as () => RouteResponse | Promise<RouteResponse>)()
      : route)
    if (isHttpError(value)) {
      return jsonResponse({ error: value.error }, value.status)
    }
    return jsonResponse(value, 200)
  })
  vi.stubGlobal('fetch', fetchMock)
  return fetchMock
}

/** A route value that never responds, keeping the query in its loading state. */
export function pendingForever(): () => Promise<RouteResponse> {
  return () => new Promise<RouteResponse>(() => {})
}

function isHttpError(value: unknown): value is { status: number; error: string } {
  return (
    typeof value === 'object' &&
    value !== null &&
    'status' in value &&
    'error' in value &&
    typeof (value as { status: unknown }).status === 'number'
  )
}

function jsonResponse(body: unknown, status: number): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

/**
 * Renders `ui` inside a fresh react-query client and a memory router, the
 * providers every routed component expects. Retries and polling are off so
 * tests stay deterministic.
 */
export function renderWithProviders(ui: ReactElement, { route = '/' }: { route?: string } = {}) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false, refetchInterval: false } },
  })
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>
      <MemoryRouter initialEntries={[route]}>{children}</MemoryRouter>
    </QueryClientProvider>
  )
  return { ...render(ui, { wrapper }), queryClient }
}

let uniqueId = 0

export function aChannel(overrides: Partial<ChannelListItem> = {}): ChannelListItem {
  uniqueId += 1
  return {
    id: `channel-${uniqueId}`,
    name: `Channel ${uniqueId}`,
    path: `channels/channel-${uniqueId}`,
    quality: 'high',
    video_limit: 3,
    avatar_filename: null,
    unwatched_count: 0,
    ...overrides,
  }
}

export function aPlaylist(overrides: Partial<PlaylistListItem> = {}): PlaylistListItem {
  uniqueId += 1
  return {
    id: `playlist-${uniqueId}`,
    name: `Playlist ${uniqueId}`,
    path: `playlists/playlist-${uniqueId}`,
    quality: 'high',
    kind: 'playlist',
    exclude_from_home: false,
    created_at: '2026-01-01T00:00:00Z',
    ...overrides,
  }
}

export function aVideo(overrides: Partial<Video> = {}): Video {
  uniqueId += 1
  return {
    id: `video-${uniqueId}`,
    title: `Video ${uniqueId}`,
    status: 'DOWNLOADED',
    quality: 'high',
    filename: `video-${uniqueId}.mp4`,
    thumbnail_filename: null,
    duration_seconds: 120,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
    watched: false,
    position_seconds: 0,
    synced_at: null,
    published_at: null,
    description: null,
    channel_name: null,
    ...overrides,
  }
}

export function aHomeVideo(overrides: Partial<HomeVideo> = {}): HomeVideo {
  uniqueId += 1
  return {
    id: `video-${uniqueId}`,
    title: `Video ${uniqueId}`,
    thumbnail_filename: null,
    duration_seconds: 120,
    watched: false,
    position_seconds: 0,
    source: {
      kind: 'playlist',
      id: `playlist-${uniqueId}`,
      name: `Playlist ${uniqueId}`,
      path: `playlists/playlist-${uniqueId}`,
      avatar_filename: null,
    },
    ...overrides,
  }
}

export function aTask(overrides: Partial<Task> = {}): Task {
  uniqueId += 1
  return {
    id: uniqueId,
    task_type: 'download_video',
    status: 'pending',
    retries: 0,
    run_at: '2026-01-01T00:00:00Z',
    created_at: '2026-01-01T00:00:00Z',
    last_error: null,
    payload: {},
    ...overrides,
  }
}

export function anAnnouncement(overrides: Partial<Announcement> = {}): Announcement {
  uniqueId += 1
  return {
    id: `announcement-${uniqueId}`,
    text: `Announcement ${uniqueId}`,
    ...overrides,
  }
}
