import { describe, expect, it, vi } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Route, Routes } from 'react-router-dom'
import { PlaylistDetail } from './PlaylistDetail'
import { aPlaylist, aVideo, mockApi, renderWithProviders, type Routes as ApiRoutes } from '@/test/helpers'

function renderPlaylist(routes: ApiRoutes, route: string) {
  mockApi(routes)
  return renderWithProviders(
    <Routes>
      <Route path="/playlists/:id" element={<PlaylistDetail />} />
      <Route path="/" element={<p>home page</p>} />
    </Routes>,
    { route },
  )
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
    // Playlists have no mark-all-watched action.
    expect(screen.queryByRole('button', { name: 'Mark all watched' })).not.toBeInTheDocument()
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

  it('offers no Edit settings item', async () => {
    renderPlaylist(
      {
        'GET /api/playlists': [aPlaylist({ id: 'PL1', name: 'The Playlist' })],
        'GET /api/playlists/PL1/videos': [],
      },
      '/playlists/PL1',
    )
    const user = userEvent.setup()

    await user.click(await screen.findByRole('button', { name: 'More actions for The Playlist' }))

    expect(await screen.findByRole('menuitem', { name: 'Delete' })).toBeInTheDocument()
    expect(screen.queryByRole('menuitem', { name: 'Edit settings' })).not.toBeInTheDocument()
  })
})
