import { describe, expect, it, vi } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Route, Routes, useNavigate, type InitialEntry } from 'react-router-dom'
import { CategoryDetail } from './CategoryDetail'
import { RouteProbe } from '@/test/RouteProbe'
import {
  aCategory,
  aCategoryList,
  aChannel,
  aHomeVideo,
  aPlaylist,
  mockApi,
  renderWithProviders,
  type Routes as ApiRoutes,
} from '@/test/helpers'

function renderCategory(routes: ApiRoutes, route: InitialEntry = '/categories/chess') {
  const fetchMock = mockApi({ 'GET /api/channels': [], 'GET /api/playlists': [], ...routes })
  const { unmount } = renderWithProviders(
    <>
      <Routes>
        <Route path="/categories/:id" element={<CategoryDetail />} />
      </Routes>
      <RouteProbe />
      <HistoryControls />
    </>,
    { route },
  )
  return { fetchMock, unmount }
}

/** Leaves the view and comes back through history, as the browser's buttons would. */
function HistoryControls() {
  const navigate = useNavigate()
  return (
    <>
      <button type="button" onClick={() => navigate('/')}>
        Go home
      </button>
      <button type="button" onClick={() => navigate(-1)}>
        Go back
      </button>
    </>
  )
}

describe('CategoryDetail', () => {
  it('shows the summary and video cards newest first', async () => {
    renderCategory({
      'GET /api/categories': aCategoryList([
        aCategory({
          id: 'chess',
          name: 'Chess',
          channels: ['@hikaru', '@gotham'],
          playlists: ['PLend'],
          unwatched_count: 1,
        }),
      ]),
      'GET /api/channels': [
        aChannel({ id: '@hikaru', name: 'Hikaru' }),
        aChannel({ id: '@gotham', name: 'GothamChess' }),
      ],
      'GET /api/playlists': [aPlaylist({ id: 'PLend', name: 'Endgame course' })],
      'GET /api/categories/chess/videos': [
        aHomeVideo({ title: 'Newest' }),
        aHomeVideo({ title: 'Older' }),
      ],
    })

    expect(await screen.findByRole('heading', { name: 'Chess' })).toBeInTheDocument()
    expect(await screen.findByText('3 sources · 2 videos · 1 unwatched')).toBeInTheDocument()
    const cards = await screen.findAllByRole('listitem')
    expect(cards.map((card) => within(card).getAllByRole('link')[1]?.textContent)).toEqual([
      'Newest',
      'Older',
    ])
  })

  it('marks all watched and deletes from the actions menu', async () => {
    let watched = false
    let category = aCategory({ id: 'chess', name: 'Chess', channels: ['@hikaru'], unwatched_count: 2 })
    const markWatched = vi.fn(() => {
      watched = true
      category = { ...category, unwatched_count: 0 }
      return null
    })
    const remove = vi.fn(() => null)
    const { fetchMock } = renderCategory({
      'GET /api/categories': () => aCategoryList([category]),
      'GET /api/channels': [aChannel({ id: '@hikaru', name: 'Hikaru' })],
      'GET /api/categories/chess/videos': () => [
        aHomeVideo({ id: 'v1', watched }),
        aHomeVideo({ id: 'v2', watched }),
      ],
      'POST /api/categories/chess/watched': markWatched,
      'DELETE /api/categories/chess': remove,
    })

    const user = userEvent.setup()
    await screen.findAllByRole('listitem')
    await user.click(screen.getByRole('button', { name: 'More actions for Chess' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Mark all watched' }))

    expect(await screen.findAllByRole('img', { name: 'Watched' })).toHaveLength(2)
    expect(markWatched).toHaveBeenCalledOnce()
    await waitFor(() =>
      expect(
        fetchMock.mock.calls.filter(([url]) => url === '/api/categories').length,
      ).toBeGreaterThan(1),
    )

    await user.click(screen.getByRole('button', { name: 'More actions for Chess' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Delete' }))
    const dialog = await screen.findByRole('dialog', { name: 'Delete "Chess"?' })
    const description = within(dialog).getByText(/holds/).textContent
    await user.click(within(dialog).getByRole('button', { name: 'Delete' }))

    expect(description).toBe(
      '"Chess" holds 1 channels and playlists. None of them, nor any video, will be deleted.',
    )
    await waitFor(() =>
      expect(screen.getByRole('status', { name: 'Current route' })).toHaveTextContent(/^\/$/),
    )
    expect(remove).toHaveBeenCalledOnce()
  })

  it('explains an empty category, whose control opens the dialog, and an unknown one', async () => {
    const routes = {
      'GET /api/categories': aCategoryList([
        aCategory({ id: 'chess', name: 'Chess' }),
        aCategory({ id: 'science', name: 'Science', channels: ['@veritasium'] }),
      ]),
      'GET /api/channels': [aChannel({ id: '@veritasium', name: 'Veritasium' })],
      'GET /api/categories/chess/videos': [],
      'GET /api/categories/science/videos': [],
      'GET /api/categories/missing/videos': { status: 404, error: 'category missing not found' },
    }
    const user = userEvent.setup()
    const empty = renderCategory(routes, '/categories/chess')
    const emptyState = await screen.findByRole('region', { name: 'Nothing in Chess yet' })
    const explanation = within(emptyState).getByText(/^Add channels and playlists to/).textContent
    await user.click(screen.getByRole('button', { name: 'More actions for Chess' }))
    const items = (await screen.findAllByRole('menuitem')).map((item) => item.textContent)
    await user.keyboard('{Escape}')
    await user.click(within(emptyState).getByRole('button', { name: 'Add channels and playlists' }))
    const dialogTitle = within(await screen.findByRole('dialog')).getByRole('heading', { level: 2 }).textContent
    empty.unmount()
    const withoutVideos = renderCategory(routes, '/categories/science')
    const noVideosMessage = (await screen.findByText(/No videos/)).textContent
    withoutVideos.unmount()

    renderCategory(routes, '/categories/missing')

    expect(explanation).toBe(
      'Add channels and playlists to Chess to watch their videos here, newest first.',
    )
    expect(items).toEqual(['Manage sources', 'Delete'])
    expect(dialogTitle).toBe('Manage sources in Chess')
    expect(noVideosMessage).toBe('No videos synced yet.')
    expect(await screen.findByText('Category not found.')).toBeInTheDocument()
  })

  it('renames from the header', async () => {
    let chess = aCategory({ id: 'chess', name: 'Chess', channels: ['@hikaru'] })
    const rename = vi.fn(() => {
      chess = { ...chess, name: 'Board games' }
      return null
    })
    const { fetchMock } = renderCategory({
      'GET /api/categories': () => aCategoryList([chess]),
      'GET /api/categories/chess/videos': [],
      'PATCH /api/categories/chess': rename,
    })
    const user = userEvent.setup()

    await user.click(await screen.findByRole('button', { name: 'Rename' }))
    await user.clear(screen.getByRole('textbox', { name: 'Name' }))
    await user.type(screen.getByRole('textbox', { name: 'Name' }), 'Board games{Enter}')

    expect(await screen.findByRole('heading', { name: 'Board games' })).toBeInTheDocument()
    const [, init] = fetchMock.mock.calls.find(([url]) => url === '/api/categories/chess') as [
      string,
      RequestInit,
    ]
    expect(JSON.parse(init.body as string)).toEqual({ name: 'Board games' })
  })

  it('opens the dialog on arrival with ManageSourcesState, once', async () => {
    renderCategory(
      {
        'GET /api/categories': aCategoryList([aCategory({ id: 'chess', name: 'Chess' })]),
        'GET /api/categories/chess/videos': [],
      },
      { pathname: '/categories/chess', state: { manageSources: true } },
    )
    const user = userEvent.setup()

    const openedOnArrival = (await screen.findByRole('dialog')).textContent?.includes(
      'Manage sources in Chess',
    )
    await user.keyboard('{Escape}')
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument())
    await user.click(screen.getByRole('button', { name: 'Go home' }))
    await user.click(screen.getByRole('button', { name: 'Go back' }))
    await screen.findByRole('region', { name: 'Nothing in Chess yet' })

    expect(openedOnArrival).toBe(true)
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
  })
})
