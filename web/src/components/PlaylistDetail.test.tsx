import { describe, expect, it, vi } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Route, Routes } from 'react-router-dom'
import { PlaylistDetail } from './PlaylistDetail'
import { aPlaylist, aVideo, mockApi, renderWithProviders, type Routes as ApiRoutes } from '@/test/helpers'

function renderPlaylist(routes: ApiRoutes, route: string) {
  const fetchMock = mockApi(routes)
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
  it('shows the playlist header, player and video list', async () => {
    const playlist = aPlaylist({ id: 'PL1', name: 'The Playlist' })
    const first = aVideo({ title: 'First Video', filename: 'first.mp4' })
    renderPlaylist(
      {
        'GET /api/playlists': [playlist],
        'GET /api/playlists/PL1/videos': [first, aVideo({ title: 'Second Video' })],
      },
      '/playlists/PL1',
    )

    expect(await screen.findByText('The Playlist')).toBeInTheDocument()
    expect(await screen.findByText('2 videos')).toBeInTheDocument()
    expect(await screen.findByRole('heading', { name: 'First Video' })).toBeInTheDocument()
    expect(document.querySelector('video')?.getAttribute('src')).toBe(
      `/media/${playlist.path}/first.mp4`,
    )
  })

  it("summarizes the playlist's unwatched count", async () => {
    for (const hidden of [false, true]) {
      const { unmount } = renderPlaylist(
        {
          'GET /api/playlists': [
            aPlaylist({ id: 'PL1', unwatched_count: 2, hide_unwatched_count: hidden }),
          ],
          'GET /api/playlists/PL1/videos': [aVideo(), aVideo(), aVideo()],
        },
        '/playlists/PL1',
      )

      expect(await screen.findByText('3 videos · 2 unwatched')).toBeInTheDocument()
      unmount()
    }
  })

  it('shows progress and time left on an in-progress row', async () => {
    renderPlaylist(
      {
        'GET /api/playlists': [aPlaylist({ id: 'PL1' })],
        'GET /api/playlists/PL1/videos': [
          aVideo({ title: 'Fresh Video', duration_seconds: 120 }),
          aVideo({ title: 'Started Video', position_seconds: 120, duration_seconds: 600 }),
        ],
      },
      '/playlists/PL1',
    )

    // Each row's select button, not its "Actions for …" menu button.
    const started = await screen.findByRole('button', { name: /^(?!Actions for).*Started Video$/ })
    expect(within(started).getByRole('progressbar', { name: 'Watch progress' })).toHaveAttribute(
      'aria-valuenow',
      '20',
    )
    expect(within(started).getByText('8m left')).toBeInTheDocument()
    const fresh = screen.getByRole('button', { name: /^(?!Actions for).*Fresh Video$/ })
    expect(within(fresh).queryByRole('progressbar')).not.toBeInTheDocument()
    expect(within(fresh).getByText('2:00')).toBeInTheDocument()
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

  it('says when the playlist has no videos yet', async () => {
    const playlist = aPlaylist({ id: 'PL1' })
    renderPlaylist(
      {
        'GET /api/playlists': [playlist],
        'GET /api/playlists/PL1/videos': [],
      },
      '/playlists/PL1',
    )

    expect(
      await screen.findByText('No videos recorded for this playlist yet.'),
    ).toBeInTheDocument()
    expect(await screen.findByText('No video selected.')).toBeInTheDocument()
  })

  it('keeps Sync visible and the other actions in the ⋮ menu', async () => {
    renderPlaylist(
      {
        'GET /api/playlists': [aPlaylist({ id: 'PL1', name: 'The Playlist' })],
        'GET /api/playlists/PL1/videos': [],
      },
      '/playlists/PL1',
    )

    expect(await screen.findByRole('button', { name: 'Sync' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Delete' })).not.toBeInTheDocument()
    const user = userEvent.setup()
    await user.click(screen.getByRole('button', { name: 'More actions for The Playlist' }))
    expect(await screen.findByRole('menuitem', { name: 'Delete' })).toBeInTheDocument()
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

    expect(await screen.findByText('2 videos · 2 unwatched')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'More actions for The Playlist' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Mark all watched' }))

    expect(await screen.findByText('2 videos')).toBeInTheDocument()
    expect(
      await screen.findByRole('button', { name: /^Watched.*Second Video$/ }),
    ).toBeInTheDocument()
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
      '/playlists/PL1',
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
        'GET /api/playlists/PL1/videos': [aVideo({ title: 'First Video', filename: 'first.mp4' })],
      },
      '/playlists/PL1',
    )
    await screen.findByRole('heading', { name: 'First Video' })
    const player = document.querySelector('video')

    await queryClient.refetchQueries({ queryKey: ['playlists'], exact: true })

    await waitFor(() => expect(playlistLoads).toBe(2))
    expect(screen.queryByText(/Failed to load playlist/)).not.toBeInTheDocument()
    expect(document.querySelector('video')).toBe(player)
  })
})
