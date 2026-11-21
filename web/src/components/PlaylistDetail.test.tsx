import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Link, Route, Routes } from 'react-router-dom'
import { PlaylistDetail } from './PlaylistDetail'
import { queryKeys } from '@/api/queries'
import {
  aCategory,
  aCategoryList,
  aPlaylist,
  aVideo,
  heldUntilReleased,
  aPreferences,
  mockApi,
  renderWithProviders,
  spyOnPlay,
  type Routes as ApiRoutes,
} from '@/test/helpers'
import { useDesktopViewport } from '@/test/viewport'

function renderPlaylist(routes: ApiRoutes, route: string) {
  const fetchMock = mockApi({
    'GET /api/preferences': aPreferences({ categories_enabled: false }),
    ...routes,
  })
  const rendered = renderWithProviders(
    <Routes>
      <Route path="/playlists/:id" element={<PlaylistDetail />} />
      <Route path="/" element={<p>home page</p>} />
    </Routes>,
    { route },
  )
  return { ...rendered, fetchMock }
}

describe('PlaylistDetail', () => {
  beforeEach(() => {
    window.localStorage.clear()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  describe('browsing', () => {
    it('shows the header, the watch-state chips and the videos as cards numbered by episode', async () => {
      vi.useFakeTimers({ toFake: ['Date'] })
      vi.setSystemTime(new Date('2026-11-15T12:00:00Z'))
      renderPlaylist(
        {
          'GET /api/playlists': [
            aPlaylist({
              id: 'PL1',
              name: 'Chess',
              unwatched_count: 2,
              last_synced_at: '2026-11-15T09:00:00Z',
            }),
          ],
          'GET /api/playlists/PL1/videos': [
            aVideo({ title: 'Pawns', episode: 1, watched: true }),
            aVideo({ title: 'Knights', episode: 2, position_seconds: 30, in_progress: true }),
            aVideo({ title: 'Bishops', episode: 3, published_at: '2026-11-13T12:00:00Z' }),
          ],
        },
        '/playlists/PL1',
      )

      expect(await screen.findByRole('heading', { name: 'Chess' })).toBeInTheDocument()
      expect(
        screen.getByText('Playlist · 3 videos · 2 unwatched · synced 3h ago'),
      ).toBeInTheDocument()
      const chips = within(screen.getByRole('group', { name: 'Show' }))
        .getAllByRole('button')
        .map((chip) => chip.textContent)
      expect(chips).toEqual(['All3', 'Unwatched2', 'In progress1', 'Watched1'])
      expect(screen.getByRole('link', { name: 'Bishops' }).closest('li')).toHaveTextContent(
        'Ep 3 · 2d ago',
      )
      expect(document.querySelector('video')).toBeNull()
    })

    it('plays next the first video in playlist order not watched yet', async () => {
      renderPlaylist(
        {
          'GET /api/playlists': [aPlaylist({ id: 'PL1' })],
          'GET /api/playlists/PL1/videos': [
            aVideo({ id: 'v1', episode: 1, watched: true }),
            aVideo({ id: 'v2', title: 'The Backward Pawn', episode: 2, position_seconds: 40 }),
            aVideo({ id: 'v3', episode: 3, watched: true }),
            aVideo({ id: 'v4', episode: 4 }),
          ],
        },
        '/playlists/PL1',
      )

      const playNext = await screen.findByRole('link', { name: /^Play next · Ep 2/ })

      expect(playNext).toHaveTextContent('Play next · Ep 2· The Backward Pawn')
      expect(playNext).toHaveAttribute('href', '/playlists/PL1?video=v2')
    })

    it('offers no Play next once every video is watched', async () => {
      renderPlaylist(
        {
          'GET /api/playlists': [aPlaylist({ id: 'PL1' })],
          'GET /api/playlists/PL1/videos': [aVideo({ title: 'Done', watched: true })],
        },
        '/playlists/PL1',
      )

      await screen.findByRole('link', { name: 'Done' })
      expect(screen.queryByRole('link', { name: /Play next/ })).not.toBeInTheDocument()
    })

    it('counts a failed episode under All only, offering no Play next for it', async () => {
      renderPlaylist(
        {
          'GET /api/playlists': [aPlaylist({ id: 'PL1', unwatched_count: 0 })],
          'GET /api/playlists/PL1/videos': [
            aVideo({ title: 'Broken', episode: 1, status: 'FAILED', filename: null }),
            aVideo({ episode: 2, watched: true }),
            aVideo({ episode: 3, watched: true }),
          ],
        },
        '/playlists/PL1',
      )

      await screen.findByRole('link', { name: 'Broken' })
      const chips = within(screen.getByRole('group', { name: 'Show' }))
        .getAllByRole('button')
        .map((chip) => chip.textContent)

      expect(screen.getByText('Playlist · 3 videos')).toBeInTheDocument()
      expect(chips).toEqual(['All3', 'Unwatched0', 'In progress0', 'Watched2'])
      expect(screen.queryByRole('link', { name: /Play next/ })).not.toBeInTheDocument()
    })

    it('filters by watch state and sorts, remembering both for the playlist', async () => {
      const routes = {
        'GET /api/playlists': [aPlaylist({ id: 'PL1' })],
        'GET /api/playlists/PL1/videos': [
          aVideo({ title: 'Old', published_at: '2020-01-01T00:00:00Z' }),
          aVideo({ title: 'Seen', watched: true, published_at: '2021-01-01T00:00:00Z' }),
          aVideo({ title: 'New', published_at: '2022-01-01T00:00:00Z' }),
        ],
      }
      const cardTitles = () =>
        screen
          .getAllByRole('listitem')
          .map((item) => within(item).getAllByRole('link')[0].textContent)
      const { unmount } = renderPlaylist(routes, '/playlists/PL1')
      const user = userEvent.setup()

      await user.click(await screen.findByRole('button', { name: /^Unwatched/ }))
      await user.click(screen.getByRole('button', { name: 'Sort: Playlist order' }))
      await user.click(await screen.findByRole('menuitemradio', { name: 'Newest first' }))

      expect(cardTitles()).toEqual(['New', 'Old'])
      unmount()

      renderPlaylist(routes, '/playlists/PL1')
      await screen.findByRole('link', { name: 'New' })
      expect([
        screen.getByRole('button', { name: /^Unwatched/ }),
        screen.getByRole('button', { name: 'Sort: Newest first' }),
      ]).toHaveLength(2)
      expect(screen.getByRole('button', { name: /^Unwatched/ })).toHaveAttribute(
        'aria-pressed',
        'true',
      )
      expect(cardTitles()).toEqual(['New', 'Old'])
    })

    it('shows the categories the playlist belongs to', async () => {
      renderPlaylist(
        {
          'GET /api/preferences': aPreferences({ categories_enabled: true }),
          'GET /api/categories': aCategoryList([
            aCategory({ id: 'c1', name: 'Chess Gameplays', playlists: ['PL1'] }),
            aCategory({ id: 'c2', name: 'Cooking', playlists: ['PL2'] }),
          ]),
          'GET /api/playlists': [aPlaylist({ id: 'PL1' })],
          'GET /api/playlists/PL1/videos': [],
        },
        '/playlists/PL1',
      )

      expect(await screen.findByRole('link', { name: 'Chess Gameplays' })).toHaveAttribute(
        'href',
        '/categories/c1',
      )
      expect(screen.queryByRole('link', { name: 'Cooking' })).not.toBeInTheDocument()
    })

    it('opens a card in the watching layout, whose name leads back', async () => {
      renderPlaylist(
        {
          'GET /api/playlists': [aPlaylist({ id: 'PL1', name: 'Chess' })],
          'GET /api/playlists/PL1/videos': [
            aVideo({ id: 'v1', title: 'Pawns', episode: 1 }),
            aVideo({ id: 'v2', title: 'Knights', episode: 2, filename: 'knights.mp4' }),
          ],
        },
        '/playlists/PL1',
      )
      const user = userEvent.setup()

      await user.click(await screen.findByRole('link', { name: 'Knights' }))

      expect(await screen.findByRole('heading', { name: 'Knights' })).toBeInTheDocument()
      expect(screen.getByText('Episode 2')).toBeInTheDocument()
      expect(document.querySelector('video')?.getAttribute('src')).toMatch(/knights\.mp4$/)
      const queue = within(screen.getByRole('complementary', { name: 'Queue' }))
      expect(queue.getByText('2 of 2 · playlist order')).toBeInTheDocument()
      expect(queue.getAllByRole('button')[0]).toHaveTextContent(/^Episode 1.*Pawns/)

      await user.click(screen.getByRole('link', { name: 'Chess' }))

      expect(await screen.findByRole('group', { name: 'Show' })).toBeInTheDocument()
      expect(document.querySelector('video')).toBeNull()
    })

    it('keeps the loaded videos when a refresh fails', async () => {
      let videoLoads = 0
      const { queryClient } = renderPlaylist(
        {
          'GET /api/playlists': [aPlaylist({ id: 'PL1' })],
          'GET /api/playlists/PL1/videos': () => {
            videoLoads += 1
            return videoLoads === 1
              ? [aVideo({ title: 'Pawns' })]
              : { status: 500, error: 'database is locked' }
          },
        },
        '/playlists/PL1',
      )
      await screen.findByRole('link', { name: 'Pawns' })

      await queryClient.refetchQueries({ queryKey: queryKeys.playlistVideos('PL1') })

      await waitFor(() => expect(videoLoads).toBe(2))
      expect(screen.queryByText(/Failed to load videos/)).not.toBeInTheDocument()
      expect(screen.getByRole('link', { name: 'Pawns' })).toBeInTheDocument()
    })

    it('says when the playlist has no videos yet', async () => {
      renderPlaylist(
        {
          'GET /api/playlists': [aPlaylist({ id: 'PL1' })],
          'GET /api/playlists/PL1/videos': [],
        },
        '/playlists/PL1',
      )

      expect(
        await screen.findByText('No videos recorded for this playlist yet.'),
      ).toBeInTheDocument()
    })
  })

  describe('watching', () => {
    it('autoplays a video opened from home that the cached list still shows downloading', async () => {
      const play = spyOnPlay()
      mockApi({
        'GET /api/preferences': aPreferences({ categories_enabled: false }),
        'GET /api/playlists': [aPlaylist({ id: 'PL1' })],
        'GET /api/playlists/PL1/videos': [aVideo({ id: 'x', title: 'Rooks', filename: 'x.mp4' })],
      })
      const { queryClient } = renderWithProviders(
        <Routes>
          <Route path="/playlists/:id" element={<PlaylistDetail />} />
          <Route path="/" element={<Link to="/playlists/PL1?video=x">Rooks</Link>} />
        </Routes>,
      )
      // Cached while the page was last open, before the download finished.
      queryClient.setQueryData(queryKeys.playlists, [aPlaylist({ id: 'PL1' })])
      queryClient.setQueryData(queryKeys.playlistVideos('PL1'), [
        aVideo({ id: 'x', title: 'Rooks', status: 'IN_PROGRESS', filename: null }),
      ])

      await userEvent.setup().click(screen.getByRole('link', { name: 'Rooks' }))

      await waitFor(() => expect(play).toHaveBeenCalledTimes(1))
      expect(play.mock.contexts[0]).toHaveAttribute('src', expect.stringMatching(/x\.mp4$/))
    })

    it('shows the playlist header, player and queue', async () => {
      const playlist = aPlaylist({ id: 'PL1', name: 'The Playlist' })
      renderPlaylist(
        {
          'GET /api/playlists': [playlist],
          'GET /api/playlists/PL1/videos': [
            aVideo({ id: 'first', title: 'First Video', filename: 'first.mp4' }),
            aVideo({ title: 'Second Video' }),
          ],
        },
        '/playlists/PL1?video=first',
      )

      expect(await screen.findByRole('link', { name: 'The Playlist' })).toHaveAttribute(
        'href',
        '/playlists/PL1',
      )
      expect(await screen.findByText('Playlist · 2 videos')).toBeInTheDocument()
      expect(await screen.findByRole('heading', { name: 'First Video' })).toBeInTheDocument()
      expect(document.querySelector('video')?.getAttribute('src')).toBe(
        `/media/${playlist.path}/first.mp4`,
      )
      expect(screen.getByText('1 of 2 · playlist order')).toBeInTheDocument()
    })

    it('hides watched videos from the queue, but the playing one, and remembers it', async () => {
      const routes = {
        'GET /api/playlists': [aPlaylist({ id: 'PL1' })],
        'GET /api/playlists/PL1/videos': [
          aVideo({ id: 'playing', title: 'Playing', watched: true }),
          aVideo({ title: 'Seen', watched: true }),
          aVideo({ title: 'Fresh' }),
        ],
      }
      const rows = () =>
        within(screen.getByRole('complementary', { name: 'Queue' }))
          .getAllByRole('listitem')
          .map((row) => within(row).getAllByRole('button')[0].textContent)
      const { unmount } = renderPlaylist(routes, '/playlists/PL1?video=playing')
      const user = userEvent.setup()

      await user.click(await screen.findByRole('switch', { name: 'Hide watched' }))

      expect(rows().map((row) => row?.replace(/.*(Playing|Seen|Fresh).*/, '$1'))).toEqual([
        'Playing',
        'Fresh',
      ])
      unmount()
      renderPlaylist(routes, '/playlists/PL1?video=playing')
      expect(await screen.findByRole('switch', { name: 'Hide watched' })).toBeChecked()
    })

    it('counts only listed rows in the queue position with watched hidden', async () => {
      renderPlaylist(
        {
          'GET /api/playlists': [aPlaylist({ id: 'PL1' })],
          'GET /api/playlists/PL1/videos': [
            aVideo({ watched: true }),
            aVideo({ watched: true }),
            aVideo({ watched: true }),
            aVideo({ id: 'playing', title: 'Playing' }),
            aVideo({ title: 'Fresh' }),
          ],
        },
        '/playlists/PL1?video=playing',
      )
      const user = userEvent.setup()
      const queue = within(await screen.findByRole('complementary', { name: 'Queue' }))

      await user.click(queue.getByRole('switch', { name: 'Hide watched' }))

      expect(queue.getByText('1 of 2 · playlist order')).toBeInTheDocument()
    })

    it('shows progress and time left on an in-progress row', async () => {
      renderPlaylist(
        {
          'GET /api/playlists': [aPlaylist({ id: 'PL1' })],
          'GET /api/playlists/PL1/videos': [
            aVideo({ id: 'fresh', title: 'Fresh Video', duration_seconds: 120 }),
            aVideo({ title: 'Started Video', position_seconds: 120, duration_seconds: 600 }),
          ],
        },
        '/playlists/PL1?video=fresh',
      )

      // Each row's select button, not its "Actions for …" menu button.
      const started = await screen.findByRole('button', {
        name: /^(?!Actions for).*Started Video$/,
      })
      expect(
        within(started).getByRole('progressbar', { name: 'Watch progress' }),
      ).toHaveAttribute('aria-valuenow', '20')
      expect(within(started).getByText('8m left')).toBeInTheDocument()
      const fresh = screen.getByRole('button', { name: /^(?!Actions for).*Fresh Video$/ })
      expect(within(fresh).queryByRole('progressbar')).not.toBeInTheDocument()
      expect(within(fresh).getByText('2:00')).toBeInTheDocument()
    })
  })

  it('reports an unknown playlist', async () => {
    renderPlaylist(
      {
        'GET /api/playlists': [],
        'GET /api/playlists/missing/videos': [],
      },
      '/playlists/missing',
    )

    expect(await screen.findByText('Playlist not found.')).toBeInTheDocument()
  })

  it('keeps every action, Sync included, in the ⋮ menu on mobile', async () => {
    renderPlaylist(
      {
        'GET /api/playlists': [aPlaylist({ id: 'PL1', name: 'The Playlist' })],
        'GET /api/playlists/PL1/videos': [],
      },
      '/playlists/PL1',
    )

    const user = userEvent.setup()
    await user.click(await screen.findByRole('button', { name: 'More actions for The Playlist' }))

    expect(
      (await screen.findAllByRole('menuitem')).map((item) => item.textContent),
    ).toEqual([
      'Sync',
      'Open playlist on YouTube',
      'Mark all watched',
      'Exclude from home',
      'Hide unwatched count',
      'Edit settings',
      'Restore discarded videos',
      'Delete',
    ])
    expect(screen.queryByRole('button', { name: 'Sync' })).not.toBeInTheDocument()
  })

  describe('on desktop', () => {
    useDesktopViewport()

    it('lays out Sync and the YouTube link as icons, the rest behind ⋮', async () => {
      renderPlaylist(
        {
          'GET /api/playlists': [aPlaylist({ id: 'PL1', name: 'The Playlist' })],
          'GET /api/playlists/PL1/videos': [],
        },
        '/playlists/PL1',
      )

      expect(await screen.findByRole('button', { name: 'Sync' })).toBeInTheDocument()
      expect(screen.getByRole('link', { name: 'Open playlist on YouTube' })).toHaveAttribute(
        'href',
        'https://www.youtube.com/playlist?list=PL1',
      )
      await userEvent
        .setup()
        .click(screen.getByRole('button', { name: 'More actions for The Playlist' }))
      expect((await screen.findAllByRole('menuitem')).map((item) => item.textContent)).toEqual([
        'Mark all watched',
        'Exclude from home',
        'Hide unwatched count',
        'Edit settings',
        'Restore discarded videos',
        'Delete',
      ])
    })
  })

  it("restores the playlist's discarded videos after confirming", async () => {
    let stored: ReturnType<typeof aVideo>[] = []
    const restore = vi.fn(() => {
      stored = [aVideo({ title: 'Restored Video', status: 'PENDING', filename: null })]
      return null
    })
    renderPlaylist(
      {
        'GET /api/playlists': [aPlaylist({ id: 'PL1', name: 'The Playlist' })],
        'GET /api/playlists/PL1/videos': () => stored,
        'POST /api/playlists/PL1/discarded/restore': restore,
      },
      '/playlists/PL1',
    )
    const user = userEvent.setup()

    await user.click(await screen.findByRole('button', { name: 'More actions for The Playlist' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Restore discarded videos' }))
    const dialog = await screen.findByRole('dialog', {
      name: 'Restore discarded videos of "The Playlist"?',
    })
    await user.click(within(dialog).getByRole('button', { name: 'Restore' }))

    expect(restore).toHaveBeenCalledOnce()
    expect(await screen.findByText('Restored Video')).toBeInTheDocument()
  })

  it('sends nothing when the restore is cancelled', async () => {
    const { fetchMock } = renderPlaylist(
      {
        'GET /api/playlists': [aPlaylist({ id: 'PL1', name: 'The Playlist' })],
        'GET /api/playlists/PL1/videos': [],
      },
      '/playlists/PL1',
    )
    const user = userEvent.setup()

    await user.click(await screen.findByRole('button', { name: 'More actions for The Playlist' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Restore discarded videos' }))
    const dialog = await screen.findByRole('dialog', {
      name: 'Restore discarded videos of "The Playlist"?',
    })
    await user.click(within(dialog).getByRole('button', { name: 'Cancel' }))

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
    expect(fetchMock.mock.calls.filter(([, init]) => init?.method === 'POST')).toEqual([])
  })

  it('excludes the playlist from home from its page header menu', async () => {
    let playlist = aPlaylist({ id: 'PL1', name: 'The Playlist' })
    const update = vi.fn(() => {
      playlist = { ...playlist, exclude_from_home: true }
      return playlist
    })
    renderPlaylist(
      {
        'GET /api/playlists': () => [playlist],
        'GET /api/playlists/PL1/videos': [],
        'PATCH /api/playlists/PL1': update,
      },
      '/playlists/PL1',
    )

    const user = userEvent.setup()
    await user.click(await screen.findByRole('button', { name: 'More actions for The Playlist' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Exclude from home' }))

    expect(update).toHaveBeenCalledOnce()
    await user.click(screen.getByRole('button', { name: 'More actions for The Playlist' }))
    expect(await screen.findByRole('menuitem', { name: 'Include in home' })).toBeInTheDocument()
  })

  it('marks the playlist watched from its page header', async () => {
    const first = aVideo({ title: 'First Video' })
    const second = aVideo({ title: 'Second Video' })
    let marked = false
    renderPlaylist(
      {
        'GET /api/playlists': () => [
          aPlaylist({ id: 'PL1', name: 'The Playlist', unwatched_count: marked ? 0 : 2 }),
        ],
        'GET /api/playlists/PL1/videos': () => [
          { ...first, watched: marked },
          { ...second, watched: marked },
        ],
        'POST /api/playlists/PL1/watched': () => {
          marked = true
          return null
        },
      },
      '/playlists/PL1',
    )
    const user = userEvent.setup()

    expect(await screen.findByText('Playlist · 2 videos · 2 unwatched')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'More actions for The Playlist' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Mark all watched' }))
    await user.click(
      within(await screen.findByRole('dialog')).getByRole('button', { name: 'Mark watched' }),
    )

    expect(await screen.findByText('Playlist · 2 videos')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /^Watched/ })).toHaveTextContent('Watched2')
  })

  it('hides the unwatched count from its page header', async () => {
    let playlist = aPlaylist({ id: 'PL1', name: 'The Playlist' })
    const update = vi.fn(() => {
      playlist = { ...playlist, hide_unwatched_count: true }
      return playlist
    })
    const { fetchMock } = renderPlaylist(
      {
        'GET /api/playlists': () => [playlist],
        'GET /api/playlists/PL1/videos': [],
        'PATCH /api/playlists/PL1': update,
      },
      '/playlists/PL1',
    )

    const user = userEvent.setup()
    await user.click(await screen.findByRole('button', { name: 'More actions for The Playlist' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Hide unwatched count' }))

    expect(update).toHaveBeenCalledOnce()
    const patchCall = fetchMock.mock.calls.find(
      ([, init]) => (init as RequestInit | undefined)?.method === 'PATCH',
    ) as [string, RequestInit]
    expect(JSON.parse(patchCall[1].body as string)).toEqual({ hide_unwatched_count: true })
    await user.click(screen.getByRole('button', { name: 'More actions for The Playlist' }))
    expect(await screen.findByRole('menuitem', { name: 'Show unwatched count' })).toBeInTheDocument()
  })

  it('deletes the playlist from its page header menu after confirming', async () => {
    const remove = vi.fn(() => null)
    renderPlaylist(
      {
        'GET /api/playlists': [aPlaylist({ id: 'PL1', name: 'The Playlist' })],
        'GET /api/playlists/PL1/videos': [],
        'DELETE /api/playlists/PL1': remove,
      },
      '/playlists/PL1',
    )
    const user = userEvent.setup()

    await user.click(await screen.findByRole('button', { name: 'More actions for The Playlist' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Delete' }))
    const dialog = await screen.findByRole('dialog')
    expect(within(dialog).getByText('Delete "The Playlist"?')).toBeInTheDocument()
    expect(remove).not.toHaveBeenCalled()
    await user.click(within(dialog).getByRole('button', { name: 'Delete' }))

    expect(await screen.findByText('home page')).toBeInTheDocument()
    expect(remove).toHaveBeenCalledOnce()
  })

  it("doesn't ask for its videos while it's being deleted", async () => {
    const videos = vi.fn(() => [])
    const remove = heldUntilReleased()
    const { queryClient } = renderPlaylist(
      {
        'GET /api/playlists': [aPlaylist({ id: 'PL1', name: 'The Playlist' })],
        'GET /api/playlists/PL1/videos': videos,
        'DELETE /api/playlists/PL1': remove.route,
      },
      '/playlists/PL1',
    )
    const user = userEvent.setup()
    await user.click(await screen.findByRole('button', { name: 'More actions for The Playlist' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Delete' }))
    await user.click(within(await screen.findByRole('dialog')).getByRole('button', { name: 'Delete' }))
    await waitFor(() => expect(remove.route).toHaveBeenCalledOnce())

    // A poll lands while the delete is in flight: the server may already have
    // removed the playlist.
    await queryClient.refetchQueries({ queryKey: queryKeys.playlistVideos('PL1') })
    remove.release()

    expect(await screen.findByText('home page')).toBeInTheDocument()
    expect(videos).toHaveBeenCalledOnce()
  })

  it('keeps asking for its videos when deleting it fails', async () => {
    const videos = vi.fn(() => [])
    const { queryClient } = renderPlaylist(
      {
        'GET /api/playlists': [aPlaylist({ id: 'PL1', name: 'The Playlist' })],
        'GET /api/playlists/PL1/videos': videos,
        'DELETE /api/playlists/PL1': { status: 500, error: 'database is locked' },
      },
      '/playlists/PL1',
    )
    const user = userEvent.setup()
    await user.click(await screen.findByRole('button', { name: 'More actions for The Playlist' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Delete' }))
    const dialog = await screen.findByRole('dialog')
    await user.click(within(dialog).getByRole('button', { name: 'Delete' }))
    expect(await within(dialog).findByText('database is locked')).toBeInTheDocument()

    await queryClient.refetchQueries({ queryKey: queryKeys.playlistVideos('PL1') })

    expect(videos).toHaveBeenCalledTimes(2)
  })

  it("alerts and leaves the playlist as it was when changing its home setting fails", async () => {
    const alert = vi.spyOn(window, 'alert').mockImplementation(() => {})
    renderPlaylist(
      {
        'GET /api/playlists': [aPlaylist({ id: 'PL1', name: 'The Playlist' })],
        'GET /api/playlists/PL1/videos': [],
        'PATCH /api/playlists/PL1': { status: 500, error: 'database is locked' },
      },
      '/playlists/PL1',
    )

    const user = userEvent.setup()
    await user.click(await screen.findByRole('button', { name: 'More actions for The Playlist' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Exclude from home' }))

    await waitFor(() =>
      expect(alert).toHaveBeenCalledWith(
        'Failed to exclude "The Playlist" from home: database is locked',
      ),
    )
    await user.click(screen.getByRole('button', { name: 'More actions for The Playlist' }))
    expect(await screen.findByRole('menuitem', { name: 'Exclude from home' })).toBeInTheDocument()
  })

  it('plays the selected video at the chosen speed and resets it for the next video', async () => {
    const playlist = aPlaylist({ id: 'PL1' })
    renderPlaylist(
      {
        'GET /api/playlists': [playlist],
        'GET /api/playlists/PL1/videos': [
          aVideo({ id: 'first', title: 'First Video', filename: 'first.mp4' }),
          aVideo({
            id: 'second',
            title: 'Second Video',
            filename: 'second.mp4',
            duration_seconds: 120,
          }),
        ],
      },
      '/playlists/PL1?video=first',
    )
    const user = userEvent.setup()

    await user.click(await screen.findByRole('button', { name: 'Playback speed' }))
    await user.click(await screen.findByRole('menuitemradio', { name: '2x' }))

    expect(document.querySelector('video')?.playbackRate).toBe(2)
    expect(screen.getByRole('button', { name: 'Playback speed' })).toHaveTextContent('2x')

    await user.click(screen.getByRole('button', { name: '2:00 Second Video' }))

    expect(await screen.findByRole('heading', { name: 'Second Video' })).toBeInTheDocument()
    expect(document.querySelector('video')?.playbackRate).toBe(1)
    expect(screen.getByRole('button', { name: 'Playback speed' })).toHaveTextContent('1x')
  })

  it("edits the playlist's settings from its page header menu", async () => {
    let playlist = aPlaylist({ id: 'PL1', name: 'The Playlist', reconcile_interval_minutes: 180 })
    const update = vi.fn(() => {
      playlist = { ...playlist, reconcile_interval_minutes: 30 }
      return playlist
    })
    renderPlaylist(
      {
        'GET /api/playlists': () => [playlist],
        'GET /api/playlists/PL1/videos': [],
        'PATCH /api/playlists/PL1': update,
      },
      '/playlists/PL1',
    )
    const user = userEvent.setup()

    await user.click(await screen.findByRole('button', { name: 'More actions for The Playlist' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Edit settings' }))
    const dialog = await screen.findByRole('dialog', { name: 'Edit The Playlist settings' })
    const amount = within(dialog).getByLabelText('Check for new videos every')
    await user.clear(amount)
    await user.type(amount, '30')
    await user.click(within(dialog).getByRole('combobox', { name: 'Interval unit' }))
    await user.click(await screen.findByRole('option', { name: 'minutes' }))
    await user.click(within(dialog).getByRole('button', { name: 'Save' }))

    await waitFor(() => expect(dialog).not.toBeInTheDocument())
    expect(update).toHaveBeenCalledOnce()
    await user.click(screen.getByRole('button', { name: 'More actions for The Playlist' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Edit settings' }))
    expect(
      within(await screen.findByRole('dialog', { name: 'Edit The Playlist settings' })).getByLabelText(
        'Check for new videos every',
      ),
    ).toHaveValue(30)
  })

  it('keeps the player when refreshing the playlist list fails after it loaded', async () => {
    const playlist = aPlaylist({ id: 'PL1', name: 'The Playlist' })
    let playlistLoads = 0
    const { queryClient } = renderPlaylist(
      {
        'GET /api/playlists': () => {
          playlistLoads += 1
          return playlistLoads === 1 ? [playlist] : { status: 500, error: 'database is locked' }
        },
        'GET /api/playlists/PL1/videos': [
          aVideo({ id: 'first', title: 'First Video', filename: 'first.mp4' }),
        ],
      },
      '/playlists/PL1?video=first',
    )
    await screen.findByRole('heading', { name: 'First Video' })
    const player = document.querySelector('video')

    await queryClient.refetchQueries({ queryKey: ['playlists'], exact: true })

    await waitFor(() => expect(playlistLoads).toBe(2))
    expect(screen.queryByText(/Failed to load playlist/)).not.toBeInTheDocument()
    expect(document.querySelector('video')).toBe(player)
  })
})
