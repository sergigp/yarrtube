import type { ReactElement, ReactNode } from 'react'
import { render } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter, type InitialEntry } from 'react-router-dom'
import { vi } from 'vitest'
import type {
  Announcement,
  AppVersion,
  Category,
  CategoryList,
  CategoryVideo,
  ChannelListItem,
  CreatedCategory,
  HomeCategorySection,
  HomeVideo,
  PlaylistListItem,
  Preferences,
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

/**
 * A route value held until `release()`, e.g. a delete still in flight; `route`
 * is a mock, so a test can wait for the request to have been sent.
 */
export function heldUntilReleased(value: RouteResponse = null): {
  route: ReturnType<typeof vi.fn>
  release: () => void
} {
  let release = () => {}
  const held = new Promise<RouteResponse>((resolve) => {
    release = () => resolve(value)
  })
  return { route: vi.fn(() => held), release: () => release() }
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
 * providers every routed component expects. `route` is a path, or a location
 * carrying navigation state. Retries and polling are off so
 * tests stay deterministic.
 */
export function renderWithProviders(
  ui: ReactElement,
  { route = '/' }: { route?: InitialEntry } = {},
) {
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
    reconcile_interval_minutes: 180,
    avatar_filename: null,
    unwatched_count: 0,
    work_in_progress: false,
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
    hide_unwatched_count: false,
    reconcile_interval_minutes: 180,
    created_at: '2026-01-01T00:00:00Z',
    unwatched_count: 0,
    work_in_progress: false,
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
    subtitle_language: null,
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
    published_at: null,
    source: {
      kind: 'playlist',
      id: `playlist-${uniqueId}`,
      name: `Playlist ${uniqueId}`,
      path: `playlists/playlist-${uniqueId}`,
      avatar_filename: null,
    },
    category_ids: [],
    ...overrides,
  }
}

export function aCategoryVideo(overrides: Partial<CategoryVideo> = {}): CategoryVideo {
  const video = aVideo()
  return {
    ...video,
    source: {
      kind: 'playlist',
      id: `playlist-${uniqueId}`,
      name: `Playlist ${uniqueId}`,
      path: `playlists/playlist-${uniqueId}`,
      avatar_filename: null,
    },
    category_ids: [],
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

export function aVersion(overrides: Partial<AppVersion> = {}): AppVersion {
  return {
    version: 'v0.9.2',
    channel: 'release',
    commit: {
      sha: 'abc1234def5678abc1234def5678abc1234def56',
      subject: 'Fix sync',
      date: '2026-10-19T10:00:00Z',
    },
    ...overrides,
  }
}

export function aCategory(overrides: Partial<Category> = {}): Category {
  uniqueId += 1
  return {
    id: `category-${uniqueId}`,
    name: `Category ${uniqueId}`,
    color: '#f59e0b',
    unwatched_count: 0,
    channels: [],
    playlists: [],
    ...overrides,
  }
}

export function aHomeCategorySection(
  overrides: Partial<HomeCategorySection> = {},
): HomeCategorySection {
  uniqueId += 1
  return {
    id: `category-${uniqueId}`,
    name: `Category ${uniqueId}`,
    color: '#f59e0b',
    videos: [aHomeVideo()],
    ...overrides,
  }
}

export function aCreatedCategory(overrides: Partial<CreatedCategory> = {}): CreatedCategory {
  uniqueId += 1
  return {
    id: `category-${uniqueId}`,
    name: `Category ${uniqueId}`,
    color: '#f59e0b',
    channels: [],
    playlists: [],
    ...overrides,
  }
}

export function aCategoryList(
  categories: Category[] = [],
  overrides: Partial<CategoryList> = {},
): CategoryList {
  return {
    categories,
    unwatched_total: 0,
    ...overrides,
  }
}

export function aPreferences(overrides: Partial<Preferences> = {}): Preferences {
  return {
    categories_enabled: true,
    home_show_continue_watching: true,
    home_show_categories: true,
    home_show_quick_watches: true,
    home_show_latest: true,
    theme: 'light',
    ...overrides,
  }
}
