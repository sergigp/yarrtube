import { beforeEach, describe, expect, it, vi } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Sidebar } from './Sidebar'
import {
  aCategoryList,
  aChannel,
  aPlaylist,
  mockApi,
  renderWithProviders,
  type Routes,
} from '@/test/helpers'

function renderSidebar(routes: Routes, { route = '/' } = {}) {
  mockApi(routes)
  return renderWithProviders(
    <Sidebar open onClose={() => {}} onAddChannel={() => {}} onAddPlaylist={() => {}}
      onAddCategory={() => {}}
    />,
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
      'GET /api/categories': aCategoryList(),
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
      'GET /api/categories': aCategoryList(),
    })

    await screen.findByText('Busy')
    // Row link text includes the unwatched badge, so match by substring.
    const names = screen.getAllByRole('link').map((link) => link.textContent ?? '')
    const busyIndex = names.findIndex((name) => name.includes('Busy'))
    const behindIndex = names.findIndex((name) => name.includes('Barely Behind'))
    expect(busyIndex).toBeGreaterThanOrEqual(0)
    expect(busyIndex).toBeLessThan(behindIndex)
  })

  it('lists every channel of an unfolded section with no show-more control', async () => {
    const channels = Array.from({ length: 40 }, (_, index) =>
      aChannel({ name: `Chan ${index}`, unwatched_count: 0 }),
    )
    renderSidebar({ 'GET /api/channels': channels, 'GET /api/playlists': [] })

    await screen.findByText('Chan 0')

    expect(
      channels.filter((channel) => screen.queryByText(channel.name) === null),
    ).toEqual([])
    expect(screen.queryByRole('button', { name: /^Show / })).not.toBeInTheDocument()
  })

  it('offers a labelled add row in an empty section and a heading + once it has items', async () => {
    renderSidebar({
      'GET /api/channels': [],
      'GET /api/playlists': [aPlaylist({ name: 'List B' })],
      'GET /api/categories': aCategoryList(),
    })

    await screen.findByText('List B')

    expect(await screen.findByRole('button', { name: 'Add category' })).toHaveTextContent(
      'Add category',
    )
    expect(screen.getByRole('button', { name: 'Add channel' })).toHaveTextContent('Add channel')
    expect(screen.getByRole('button', { name: 'Add playlist' })).toHaveTextContent('')
    expect(screen.getByRole('button', { name: 'Add playlist' })).toHaveAttribute(
      'title',
      'Add playlist',
    )
  })

  it('folds a section from its heading, hiding its rows and add control', async () => {
    renderSidebar({
      'GET /api/channels': [aChannel({ name: 'Chan A' })],
      'GET /api/playlists': [],
      'GET /api/categories': aCategoryList(),
    })

    await screen.findByText('Chan A')
    await userEvent.setup().click(screen.getByRole('button', { name: 'Channels' }))

    expect(screen.getByRole('heading', { name: 'Channels' })).toBeInTheDocument()
    expect(screen.queryByText('Chan A')).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Add channel' })).not.toBeInTheDocument()
  })

  it('unfolds a folded section', async () => {
    renderSidebar({
      'GET /api/channels': [aChannel({ name: 'Chan A' })],
      'GET /api/playlists': [],
      'GET /api/categories': aCategoryList(),
    })

    await screen.findByText('Chan A')
    const user = userEvent.setup()
    await user.click(screen.getByRole('button', { name: 'Channels' }))
    await user.click(screen.getByRole('button', { name: 'Channels' }))

    expect(screen.getByText('Chan A')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Add channel' })).toBeInTheDocument()
  })

  it('folds sections independently', async () => {
    renderSidebar({
      'GET /api/channels': [aChannel({ name: 'Chan A' })],
      'GET /api/playlists': [aPlaylist({ name: 'List B' })],
      'GET /api/categories': aCategoryList(),
    })

    await screen.findByText('Chan A')
    await userEvent.setup().click(screen.getByRole('button', { name: 'Channels' }))

    expect(await screen.findByText('List B')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Add playlist' })).toBeInTheDocument()
  })

  it('exposes the fold state on the heading control', async () => {
    renderSidebar({ 'GET /api/channels': [], 'GET /api/playlists': [] })

    const heading = await screen.findByRole('button', { name: 'Channels' })
    const unfolded = heading.getAttribute('aria-expanded')
    await userEvent.setup().click(heading)

    expect([unfolded, heading.getAttribute('aria-expanded')]).toEqual(['true', 'false'])
  })

  it('remembers a folded section across remounts', async () => {
    const routes = {
      'GET /api/channels': [aChannel({ name: 'Chan A' })],
      'GET /api/playlists': [aPlaylist({ name: 'List B' })],
      'GET /api/categories': aCategoryList(),
    }
    const { unmount } = renderSidebar(routes)
    await screen.findByText('Chan A')
    await userEvent.setup().click(screen.getByRole('button', { name: 'Playlists' }))
    unmount()

    renderSidebar(routes)

    expect(await screen.findByText('Chan A')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Playlists' })).toHaveAttribute(
      'aria-expanded',
      'false',
    )
    expect(screen.queryByText('List B')).not.toBeInTheDocument()
  })

  it("shows the channels' unwatched total badge only while folded and above 0", async () => {
    const { unmount } = renderSidebar({
      'GET /api/channels': [
        aChannel({ name: 'Chan A', unwatched_count: 3 }),
        aChannel({ name: 'Chan B', unwatched_count: 0 }),
        aChannel({ name: 'Chan C', unwatched_count: 20 }),
      ],
      'GET /api/playlists': [],
      'GET /api/categories': aCategoryList(),
    })
    await screen.findByText('Chan A')
    const totalWhileUnfolded = screen.queryByLabelText('23 unwatched')
    await userEvent.setup().click(screen.getByRole('button', { name: 'Channels' }))
    const totalWhileFolded = screen.queryByLabelText('23 unwatched')?.textContent
    unmount()

    renderSidebar({
      'GET /api/channels': [aChannel({ name: 'Chan A', unwatched_count: 0 })],
      'GET /api/playlists': [],
      'GET /api/categories': aCategoryList(),
    })
    const caughtUpHeading = await screen.findByRole('button', { name: 'Channels' })

    expect([totalWhileUnfolded, totalWhileFolded]).toEqual([null, '23'])
    expect(caughtUpHeading).toHaveAttribute('aria-expanded', 'false')
    expect(caughtUpHeading).toHaveTextContent(/^Channels$/)
  })

  it('shows no total on a folded Playlists heading', async () => {
    renderSidebar({
      'GET /api/channels': [],
      'GET /api/playlists': [aPlaylist({ name: 'List B' })],
      'GET /api/categories': aCategoryList(),
    })

    await screen.findByText('List B')
    const heading = screen.getByRole('button', { name: 'Playlists' })
    await userEvent.setup().click(heading)

    expect(heading).toHaveAttribute('aria-expanded', 'false')
    expect(heading).toHaveTextContent(/^Playlists$/)
  })

  it("keeps a folded section folded and marks its heading active on that section's detail route", async () => {
    renderSidebar(
      {
        'GET /api/channels': [aChannel({ id: 'chan', name: 'Chan A' })],
        'GET /api/playlists': [aPlaylist({ name: 'List B' })],
        'GET /api/categories': aCategoryList(),
      },
      { route: '/channels/chan' },
    )

    await screen.findByText('Chan A')
    await userEvent.setup().click(screen.getByRole('button', { name: 'Channels' }))
    const channelsHeading = screen.getByRole('button', { name: 'Channels' })

    expect(channelsHeading).toHaveAttribute('aria-expanded', 'false')
    expect(channelsHeading).toHaveAttribute('aria-current', 'true')
    expect(screen.queryByText('Chan A')).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Playlists' })).not.toHaveAttribute('aria-current')
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

  it('shows matches of a folded section while searching, and folds it again when cleared', async () => {
    const channels = [
      aChannel({ name: 'Veritasium' }),
      ...Array.from({ length: 15 }, (_, index) => aChannel({ name: `Chan ${index}` })),
    ]
    renderSidebar({ 'GET /api/channels': channels, 'GET /api/playlists': [] })

    const search = await screen.findByRole('searchbox', { name: 'Search channels and playlists' })
    const user = userEvent.setup()
    await user.click(screen.getByRole('button', { name: 'Channels' }))
    await user.type(search, 'ver')
    const matchWhileSearching = screen.queryByText('Veritasium')?.textContent
    await user.clear(search)

    expect(matchWhileSearching).toBe('Veritasium')
    expect(screen.queryByText('Veritasium')).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Channels' })).toHaveAttribute(
      'aria-expanded',
      'false',
    )
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
      'GET /api/categories': aCategoryList(),
    })

    expect(await screen.findByText('Failed to load: boom')).toBeInTheDocument()
  })

  it('deletes a playlist after confirmation', async () => {
    const playlist = aPlaylist({ name: 'Doomed' })
    let playlistRows = [playlist]
    renderSidebar({
      'GET /api/channels': [],
      'GET /api/playlists': () => playlistRows,
      'GET /api/categories': aCategoryList(),
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
      'GET /api/categories': aCategoryList(),
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
      'GET /api/categories': aCategoryList(),
      [`POST /api/channels/${channel.id}/watched`]: markWatched,
    })

    const user = userEvent.setup()
    await user.click(await screen.findByRole('button', { name: 'Actions for Chan A' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Mark all watched' }))

    expect(markWatched).toHaveBeenCalledOnce()
  })

  it('a playlist row menu offers no mark-all-watched item', async () => {
    renderSidebar({
      'GET /api/channels': [],
      'GET /api/playlists': [aPlaylist({ name: 'List B' })],
      'GET /api/categories': aCategoryList(),
    })

    const user = userEvent.setup()
    await user.click(await screen.findByRole('button', { name: 'Actions for List B' }))

    expect(
      (await screen.findAllByRole('menuitem')).map((item) => item.textContent),
    ).toEqual(['Sync', 'Exclude from home', 'Edit settings', 'Delete'])
  })

  it("edits a playlist's settings from its row menu", async () => {
    let playlist = aPlaylist({ id: 'PL1', name: 'Lofi beats', reconcile_interval_minutes: 180 })
    const update = vi.fn(() => {
      playlist = { ...playlist, reconcile_interval_minutes: 30 }
      return playlist
    })
    const fetchMock = mockApi({
      'GET /api/channels': [],
      'GET /api/playlists': () => [playlist],
      'GET /api/categories': aCategoryList(),
      'PATCH /api/playlists/PL1': update,
    })
    renderWithProviders(
      <Sidebar open onClose={() => {}} onAddChannel={() => {}} onAddPlaylist={() => {}}
      onAddCategory={() => {}}
    />,
    )

    const user = userEvent.setup()
    await user.click(await screen.findByRole('button', { name: 'Actions for Lofi beats' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Edit settings' }))
    const dialog = await screen.findByRole('dialog', { name: 'Edit Lofi beats settings' })
    const amount = within(dialog).getByLabelText('Check for new videos every')
    await user.clear(amount)
    await user.type(amount, '30')
    await user.click(within(dialog).getByRole('combobox', { name: 'Interval unit' }))
    await user.click(await screen.findByRole('option', { name: 'minutes' }))
    await user.click(within(dialog).getByRole('button', { name: 'Save' }))

    await waitFor(() => expect(dialog).not.toBeInTheDocument())
    expect(update).toHaveBeenCalledOnce()
    const patchCall = fetchMock.mock.calls.find(
      ([, init]) => (init as RequestInit | undefined)?.method === 'PATCH',
    ) as [string, RequestInit]
    expect(JSON.parse(patchCall[1].body as string)).toEqual({ reconcile_interval_minutes: 30 })
    expect(fetchMock.mock.calls.some(([url]) => url === '/api/playlists/PL1/reconcile')).toBe(false)
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
      'GET /api/categories': aCategoryList(),
      'PATCH /api/playlists/PL1': update,
    })
    renderWithProviders(
      <Sidebar open onClose={() => {}} onAddChannel={() => {}} onAddPlaylist={() => {}}
      onAddCategory={() => {}}
    />,
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
      'GET /api/categories': aCategoryList(),
      'PATCH /api/playlists/PL1': update,
    })
    renderWithProviders(
      <Sidebar open onClose={() => {}} onAddChannel={() => {}} onAddPlaylist={() => {}}
      onAddCategory={() => {}}
    />,
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
      'GET /api/categories': aCategoryList(),
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
      'GET /api/categories': aCategoryList(),
      'PATCH /api/channels/chan': update,
    })
    renderWithProviders(
      <Sidebar open onClose={() => {}} onAddChannel={() => {}} onAddPlaylist={() => {}}
      onAddCategory={() => {}}
    />,
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
      'GET /api/categories': aCategoryList(),
      'PATCH /api/channels/chan': aChannel({
        id: 'chan',
        name: 'Chan A',
        video_limit: 20,
      }),
      'POST /api/channels/chan/reconcile': reconcile,
    })
    renderWithProviders(
      <Sidebar open onClose={() => {}} onAddChannel={() => {}} onAddPlaylist={() => {}}
      onAddCategory={() => {}}
    />,
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
      'GET /api/categories': aCategoryList(),
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
    mockApi({
      'GET /api/channels': [],
      'GET /api/playlists': [],
      'GET /api/categories': aCategoryList(),
    })
    const onClose = vi.fn()
    renderWithProviders(
      <Sidebar open onClose={onClose} onAddChannel={() => {}} onAddPlaylist={() => {}}
      onAddCategory={() => {}}
    />,
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
