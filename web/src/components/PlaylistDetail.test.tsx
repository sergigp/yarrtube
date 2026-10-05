import { describe, expect, it } from 'vitest'
import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Route, Routes } from 'react-router-dom'
import { PlaylistDetail } from './PlaylistDetail'
import { aPlaylist, aVideo, mockApi, renderWithProviders, type Routes as ApiRoutes } from '@/test/helpers'

function renderPlaylist(routes: ApiRoutes, route: string) {
  mockApi(routes)
  return renderWithProviders(
    <Routes>
      <Route path="/playlists/:id" element={<PlaylistDetail />} />
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
    await user.click(screen.getByRole('button', { name: 'Actions for The Playlist' }))
    expect(await screen.findByRole('menuitem', { name: 'Delete' })).toBeInTheDocument()
  })
})
