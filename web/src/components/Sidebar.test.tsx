import { beforeEach, describe, expect, it, vi } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Sidebar } from './Sidebar'
import { aChannel, aPlaylist, mockApi, renderWithProviders, type Routes } from '@/test/helpers'

function renderSidebar(routes: Routes, { route = '/' } = {}) {
  mockApi(routes)
  return renderWithProviders(
    <Sidebar open onClose={() => {}} onAddChannel={() => {}} onAddPlaylist={() => {}} />,
    { route },
  )
}

describe('Sidebar', () => {
  beforeEach(() => {
    window.localStorage.clear()
  })

  it('lists channels and playlists with unwatched badges', async () => {
    renderSidebar({
      'GET /api/channels': [aChannel({ name: 'Chan A', unwatched_count: 3 })],
      'GET /api/playlists': [aPlaylist({ name: 'List B' })],
    })

    expect(await screen.findByText('Chan A')).toBeInTheDocument()
    expect(await screen.findByText('List B')).toBeInTheDocument()
    expect(screen.getByTitle('3 unwatched')).toHaveTextContent('3')
  })

  it('orders channels by unwatched count', async () => {
    renderSidebar({
      'GET /api/channels': [
        aChannel({ name: 'Barely Behind', unwatched_count: 1 }),
        aChannel({ name: 'Busy', unwatched_count: 5 }),
      ],
      'GET /api/playlists': [],
    })

    await screen.findByText('Busy')
    // Row link text includes the unwatched badge, so match by substring.
    const names = screen.getAllByRole('link').map((link) => link.textContent ?? '')
    const busyIndex = names.findIndex((name) => name.includes('Busy'))
    const behindIndex = names.findIndex((name) => name.includes('Barely Behind'))
    expect(busyIndex).toBeGreaterThanOrEqual(0)
    expect(busyIndex).toBeLessThan(behindIndex)
  })

  it('collapses long caught-up sections behind a "show more" toggle', async () => {
    const channels = Array.from({ length: 8 }, (_, index) =>
      aChannel({ name: `Channel ${String.fromCharCode(65 + index)}`, unwatched_count: 0 }),
    )
    renderSidebar({ 'GET /api/channels': channels, 'GET /api/playlists': [] })

    await screen.findByText('Channel A')

    // Five caught-up channels lead; the other three hide behind the toggle.
    expect(screen.queryByText('Channel F')).not.toBeInTheDocument()
    const user = userEvent.setup()
    await user.click(screen.getByRole('button', { name: 'Show 3 more' }))
    expect(screen.getByText('Channel F')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Show less' }))
    expect(screen.queryByText('Channel F')).not.toBeInTheDocument()
  })

  it('offers search only above the threshold and filters both sections', async () => {
    const channels = Array.from({ length: 10 }, (_, index) =>
      aChannel({ name: `Chan ${index}`, unwatched_count: 1 }),
    )
    const playlists = Array.from({ length: 10 }, (_, index) => aPlaylist({ name: `List ${index}` }))
    renderSidebar({ 'GET /api/channels': channels, 'GET /api/playlists': playlists })

    const search = await screen.findByRole('searchbox', { name: 'Search channels and playlists' })
    const user = userEvent.setup()
    await user.type(search, 'List 3')

    expect(screen.getByText('List 3')).toBeInTheDocument()
    expect(screen.queryByText('Chan 3')).not.toBeInTheDocument()
    // The channels section disappears entirely once nothing in it matches.
    expect(screen.queryByText('Channels')).not.toBeInTheDocument()
  })

  it('says when nothing matches the search', async () => {
    const channels = Array.from({ length: 16 }, (_, index) => aChannel({ name: `Chan ${index}` }))
    renderSidebar({ 'GET /api/channels': channels, 'GET /api/playlists': [] })

    const search = await screen.findByRole('searchbox', { name: 'Search channels and playlists' })
    await userEvent.setup().type(search, 'zzz')

    expect(screen.getByText('Nothing matches "zzz".')).toBeInTheDocument()
  })

  it('hides the search box for small libraries', async () => {
    renderSidebar({ 'GET /api/channels': [aChannel()], 'GET /api/playlists': [] })

    await screen.findByText('Channels')
    expect(screen.queryByRole('searchbox')).not.toBeInTheDocument()
  })

  it('reports section load failures', async () => {
    renderSidebar({
      'GET /api/channels': { status: 500, error: 'boom' },
      'GET /api/playlists': [],
    })

    expect(await screen.findByText('Failed to load: boom')).toBeInTheDocument()
  })

  it('deletes a playlist after confirmation', async () => {
    const playlist = aPlaylist({ name: 'Doomed' })
    let playlistRows = [playlist]
    renderSidebar({
      'GET /api/channels': [],
      'GET /api/playlists': () => playlistRows,
      [`DELETE /api/playlists/${playlist.id}`]: () => {
        playlistRows = []
        return null
      },
    })

    const user = userEvent.setup()
    await user.click(await screen.findByRole('button', { name: 'Actions for Doomed' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Delete' }))

    const dialog = await screen.findByRole('dialog')
    expect(within(dialog).getByText('Delete "Doomed"?')).toBeInTheDocument()
    await user.click(within(dialog).getByRole('button', { name: 'Delete' }))

    await waitFor(() => expect(screen.queryByText('Doomed')).not.toBeInTheDocument())
  })

  it('syncs a channel from its row menu', async () => {
    const channel = aChannel({ name: 'Chan A' })
    const reconcile = vi.fn(() => null)
    renderSidebar({
      'GET /api/channels': [channel],
      'GET /api/playlists': [],
      [`POST /api/channels/${channel.id}/reconcile`]: reconcile,
    })

    const user = userEvent.setup()
    await user.click(await screen.findByRole('button', { name: 'Actions for Chan A' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Sync' }))

    expect(reconcile).toHaveBeenCalledOnce()
  })

  it('marks a channel watched from its row menu', async () => {
    const channel = aChannel({ name: 'Chan A', unwatched_count: 2 })
    const markWatched = vi.fn(() => null)
    renderSidebar({
      'GET /api/channels': [channel],
      'GET /api/playlists': [],
      [`POST /api/channels/${channel.id}/watched`]: markWatched,
    })

    const user = userEvent.setup()
    await user.click(await screen.findByRole('button', { name: 'Actions for Chan A' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Mark all watched' }))

    expect(markWatched).toHaveBeenCalledOnce()
  })

  it('does not offer mark-all-watched for playlists', async () => {
    renderSidebar({
      'GET /api/channels': [],
      'GET /api/playlists': [aPlaylist({ name: 'List B' })],
    })

    const user = userEvent.setup()
    await user.click(await screen.findByRole('button', { name: 'Actions for List B' }))

    expect(await screen.findByRole('menuitem', { name: 'Sync' })).toBeInTheDocument()
    expect(screen.queryByRole('menuitem', { name: 'Mark all watched' })).not.toBeInTheDocument()
    expect(screen.queryByRole('menuitem', { name: 'Edit settings' })).not.toBeInTheDocument()
  })

  it('excludes a playlist from home from its row menu', async () => {
    let playlist = aPlaylist({ id: 'PL1', name: 'Bluey' })
    const update = vi.fn(() => {
      playlist = { ...playlist, exclude_from_home: true }
      return playlist
    })
    const fetchMock = mockApi({
      'GET /api/channels': [],
      'GET /api/playlists': () => [playlist],
      'PATCH /api/playlists/PL1': update,
    })
    renderWithProviders(
      <Sidebar open onClose={() => {}} onAddChannel={() => {}} onAddPlaylist={() => {}} />,
    )

    const user = userEvent.setup()
    await user.click(await screen.findByRole('button', { name: 'Actions for Bluey' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Exclude from home' }))

    expect(update).toHaveBeenCalledOnce()
    const patchCall = fetchMock.mock.calls.find(
      ([, init]) => (init as RequestInit | undefined)?.method === 'PATCH',
    ) as [string, RequestInit]
    expect(JSON.parse(patchCall[1].body as string)).toEqual({ exclude_from_home: true })
    await user.click(await screen.findByRole('button', { name: 'Actions for Bluey' }))
    expect(await screen.findByRole('menuitem', { name: 'Include in home' })).toBeInTheDocument()
  })

  it('includes an excluded playlist in home from its row menu', async () => {
    let playlist = aPlaylist({ id: 'PL1', name: 'Bluey', exclude_from_home: true })
    const update = vi.fn(() => {
      playlist = { ...playlist, exclude_from_home: false }
      return playlist
    })
    const fetchMock = mockApi({
      'GET /api/channels': [],
      'GET /api/playlists': () => [playlist],
      'PATCH /api/playlists/PL1': update,
    })
    renderWithProviders(
      <Sidebar open onClose={() => {}} onAddChannel={() => {}} onAddPlaylist={() => {}} />,
    )

    const user = userEvent.setup()
    await user.click(await screen.findByRole('button', { name: 'Actions for Bluey' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Include in home' }))

    expect(update).toHaveBeenCalledOnce()
    const patchCall = fetchMock.mock.calls.find(
      ([, init]) => (init as RequestInit | undefined)?.method === 'PATCH',
    ) as [string, RequestInit]
    expect(JSON.parse(patchCall[1].body as string)).toEqual({ exclude_from_home: false })
    await user.click(await screen.findByRole('button', { name: 'Actions for Bluey' }))
    expect(await screen.findByRole('menuitem', { name: 'Exclude from home' })).toBeInTheDocument()
  })

  it('a channel row menu offers no home item', async () => {
    renderSidebar({
      'GET /api/channels': [aChannel({ name: 'Chan A' })],
      'GET /api/playlists': [],
    })

    const user = userEvent.setup()
    await user.click(await screen.findByRole('button', { name: 'Actions for Chan A' }))

    expect(
      (await screen.findAllByRole('menuitem')).map((item) => item.textContent),
    ).toEqual(['Sync', 'Mark all watched', 'Edit settings', 'Delete'])
  })

  it("edits a channel's settings from its row menu", async () => {
    let channel = aChannel({
      id: 'chan',
      name: 'Chan A',
      quality: 'high',
      video_limit: 5,
    })
    const update = vi.fn(() => {
      channel = { ...channel, quality: 'low' }
      return channel
    })
    const fetchMock = mockApi({
      'GET /api/channels': () => [channel],
      'GET /api/playlists': [],
      'PATCH /api/channels/chan': update,
    })
    renderWithProviders(
      <Sidebar open onClose={() => {}} onAddChannel={() => {}} onAddPlaylist={() => {}} />,
    )

    const user = userEvent.setup()
    await user.click(await screen.findByRole('button', { name: 'Actions for Chan A' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Edit settings' }))
    const dialog = await screen.findByRole('dialog', {
      name: 'Edit Chan A settings',
    })
    await user.click(within(dialog).getByRole('combobox', { name: /Video quality/ }))
    await user.click(await screen.findByRole('option', { name: 'Low' }))
    await user.click(within(dialog).getByRole('button', { name: 'Save' }))

    await waitFor(() => expect(dialog).not.toBeInTheDocument())
    expect(update).toHaveBeenCalledOnce()
    const patchCall = fetchMock.mock.calls.find(
      ([, init]) => (init as RequestInit | undefined)?.method === 'PATCH',
    ) as [string, RequestInit]
    expect(JSON.parse(patchCall[1].body as string)).toEqual({ quality: 'low' })
    expect(fetchMock.mock.calls.some(([url]) => url === '/api/channels/chan/reconcile')).toBe(false)
  })

  it('syncs a channel after its video limit changes from its row menu', async () => {
    const reconcile = vi.fn(() => null)
    mockApi({
      'GET /api/channels': [aChannel({ id: 'chan', name: 'Chan A', video_limit: 5 })],
      'GET /api/playlists': [],
      'PATCH /api/channels/chan': aChannel({
        id: 'chan',
        name: 'Chan A',
        video_limit: 20,
      }),
      'POST /api/channels/chan/reconcile': reconcile,
    })
    renderWithProviders(
      <Sidebar open onClose={() => {}} onAddChannel={() => {}} onAddPlaylist={() => {}} />,
    )

    const user = userEvent.setup()
    await user.click(await screen.findByRole('button', { name: 'Actions for Chan A' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Edit settings' }))
    const dialog = await screen.findByRole('dialog', {
      name: 'Edit Chan A settings',
    })
    const limit = within(dialog).getByLabelText('Video limit')
    await user.clear(limit)
    await user.type(limit, '20')
    await user.click(within(dialog).getByRole('button', { name: 'Save' }))

    await waitFor(() => expect(reconcile).toHaveBeenCalledOnce())
    expect(dialog).not.toBeInTheDocument()
  })

  it("alerts when changing a playlist's home setting fails", async () => {
    const alert = vi.spyOn(window, 'alert').mockImplementation(() => {})
    renderSidebar({
      'GET /api/channels': [],
      'GET /api/playlists': [aPlaylist({ id: 'PL1', name: 'Bluey' })],
      'PATCH /api/playlists/PL1': { status: 500, error: 'database is locked' },
    })

    const user = userEvent.setup()
    await user.click(await screen.findByRole('button', { name: 'Actions for Bluey' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Exclude from home' }))

    await waitFor(() =>
      expect(alert).toHaveBeenCalledWith(
        'Failed to exclude "Bluey" from home: database is locked',
      ),
    )
    await user.click(await screen.findByRole('button', { name: 'Actions for Bluey' }))
    expect(await screen.findByRole('menuitem', { name: 'Exclude from home' })).toBeInTheDocument()
  })

  it('closes when Escape is pressed while open', async () => {
    mockApi({ 'GET /api/channels': [], 'GET /api/playlists': [] })
    const onClose = vi.fn()
    renderWithProviders(
      <Sidebar open onClose={onClose} onAddChannel={() => {}} onAddPlaylist={() => {}} />,
    )

    const user = userEvent.setup()
    await user.keyboard('{Escape}')

    expect(onClose).toHaveBeenCalledOnce()
  })

  it('has no title or close button of its own', async () => {
    renderSidebar({ 'GET /api/channels': [], 'GET /api/playlists': [] })

    expect(await screen.findByRole('heading', { name: 'Channels' })).toBeInTheDocument()
    expect(screen.queryByText('Menu')).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Close menu' })).not.toBeInTheDocument()
  })
})
