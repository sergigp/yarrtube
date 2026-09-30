import { describe, expect, it, vi } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { AddPlaylistDialog } from './AddPlaylistDialog'
import { aPlaylist, mockApi, renderWithProviders, type Routes } from '@/test/helpers'

const baseRoutes: Routes = {
  'GET /api/playlists': [],
  'GET /api/channels': [],
  'GET /api/directories': { root: '/videos', path: '', entries: [] },
  'GET /api/directories?path=playlists': { root: '/videos', path: 'playlists', entries: [] },
}

function renderDialog(routes: Routes) {
  const fetchMock = mockApi({ ...baseRoutes, ...routes })
  const onOpenChange = vi.fn()
  renderWithProviders(<AddPlaylistDialog open onOpenChange={onOpenChange} />)
  return { fetchMock, onOpenChange }
}

describe('AddPlaylistDialog', () => {
  it('shows the save-to list with the default parent selected on open', async () => {
    renderDialog({})

    expect(await screen.findByRole('radio', { name: /playlists\// })).toBeChecked()
    expect(screen.getByRole('button', { name: 'Choose another folder…' })).toBeInTheDocument()
  })

  it('suggests parent folders of tracked playlists with their counts', async () => {
    renderDialog({
      'GET /api/playlists': [
        aPlaylist({ path: 'playlists/kids/contes' }),
        aPlaylist({ path: 'playlists/kids/fa-la-la' }),
      ],
    })

    const kids = await screen.findByRole('radio', { name: 'playlists/kids/' })
    expect(kids).not.toBeChecked()
    expect(screen.getByText('· 2 items')).toBeInTheDocument()
    expect(screen.getByRole('radio', { name: 'playlists/', checked: true })).toBeInTheDocument()
  })

  it('selects a suggested folder in one click and the notice follows', async () => {
    renderDialog({
      'GET /api/playlists': [aPlaylist({ path: 'playlists/kids/contes' })],
      'GET /api/playlists/preview?playlist=PL1': { id: 'PL1', title: 'My Mix', video_count: 3 },
    })
    const user = userEvent.setup()

    await user.type(screen.getByLabelText('Playlist ID or URL'), 'PL1')
    expect(await screen.findByText('/videos/playlists/my-mix')).toBeInTheDocument()

    await user.click(screen.getByRole('radio', { name: 'playlists/kids/' }))

    expect(await screen.findByText('/videos/playlists/kids/my-mix')).toBeInTheDocument()
  })

  it('looks the entered playlist up and states the destination', async () => {
    renderDialog({
      'GET /api/playlists/preview?playlist=PL1': { id: 'PL1', title: 'My Mix', video_count: 3 },
    })
    const user = userEvent.setup()

    await user.type(screen.getByLabelText('Playlist ID or URL'), 'PL1')

    expect(await screen.findByText('Looking up playlist…')).toBeInTheDocument()
    expect(
      await screen.findByText(/All 3 videos from “My Mix” will be downloaded to/),
    ).toBeInTheDocument()
    expect(screen.getByText('/videos/playlists/my-mix')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Create Playlist' })).toBeEnabled()
  })

  it('creates the playlist and closes on submit', async () => {
    const { fetchMock, onOpenChange } = renderDialog({
      'GET /api/playlists/preview?playlist=PL1': { id: 'PL1', title: 'My Mix', video_count: 3 },
      'POST /api/playlists': aPlaylist(),
    })
    const user = userEvent.setup()

    await user.type(screen.getByLabelText('Playlist ID or URL'), 'PL1')
    const submit = screen.getByRole('button', { name: 'Create Playlist' })
    await waitFor(() => expect(submit).toBeEnabled())
    await user.click(submit)

    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false))
    const createCall = fetchMock.mock.calls.find(
      ([, init]) => (init as RequestInit | undefined)?.method === 'POST',
    ) as [string, RequestInit]
    expect(JSON.parse(createCall[1].body as string)).toEqual({
      playlist: 'PL1',
      path: 'playlists/my-mix',
      quality: 'high',
    })
  })

  it('blocks a playlist that is already tracked', async () => {
    renderDialog({
      'GET /api/playlists': [aPlaylist({ id: 'PL1', name: 'Existing Mix' })],
      'GET /api/playlists/preview?playlist=PL1': { id: 'PL1', title: 'My Mix', video_count: 3 },
    })
    const user = userEvent.setup()

    await user.type(screen.getByLabelText('Playlist ID or URL'), 'PL1')

    expect(await screen.findByText('Already added as “Existing Mix”')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Create Playlist' })).toBeDisabled()
  })

  it('shows the lookup failure and keeps submission blocked', async () => {
    renderDialog({
      'GET /api/playlists/preview?playlist=nope': { status: 404, error: 'playlist not found' },
    })
    const user = userEvent.setup()

    await user.type(screen.getByLabelText('Playlist ID or URL'), 'nope')

    expect(await screen.findByText('playlist not found')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Create Playlist' })).toBeDisabled()
  })

  it('shows a creation failure and stays open', async () => {
    const { onOpenChange } = renderDialog({
      'GET /api/playlists/preview?playlist=PL1': { id: 'PL1', title: 'My Mix', video_count: 3 },
      'POST /api/playlists': { status: 422, error: 'destination already used' },
    })
    const user = userEvent.setup()

    await user.type(screen.getByLabelText('Playlist ID or URL'), 'PL1')
    const submit = screen.getByRole('button', { name: 'Create Playlist' })
    await waitFor(() => expect(submit).toBeEnabled())
    await user.click(submit)

    expect(await screen.findByText('destination already used')).toBeInTheDocument()
    expect(onOpenChange).not.toHaveBeenCalledWith(false)
  })

  it('flags a destination already used by another entry', async () => {
    renderDialog({
      'GET /api/playlists': [aPlaylist({ id: 'other', name: 'Other Mix', path: 'playlists/my-mix' })],
      'GET /api/playlists/preview?playlist=PL1': { id: 'PL1', title: 'My Mix', video_count: 3 },
    })
    const user = userEvent.setup()

    await user.type(screen.getByLabelText('Playlist ID or URL'), 'PL1')

    expect(
      await screen.findByText(/is already used by Other Mix\. Choose a different folder\./),
    ).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Create Playlist' })).toBeDisabled()
  })
})
