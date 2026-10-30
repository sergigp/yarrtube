import { describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Route, Routes, useNavigate, type InitialEntry } from 'react-router-dom'
import { CategoryDetail } from './CategoryDetail'
import { softTint } from '@/lib/categoryColors'
import { RouteProbe } from '@/test/RouteProbe'
import {
  aCategory,
  aCategoryList,
  aCategoryVideo,
  aChannel,
  aHomeVideo,
  aPlaylist,
  aPreferences,
  mockApi,
  pendingForever,
  renderWithProviders,
  type Routes as ApiRoutes,
} from '@/test/helpers'

function renderCategory(routes: ApiRoutes, route: InitialEntry = '/categories/chess') {
  const fetchMock = mockApi({
    'GET /api/channels': [],
    'GET /api/playlists': [],
    'GET /api/preferences': aPreferences(),
    ...routes,
  })
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
  it('says categories are turned off while they are', async () => {
    renderCategory({
      'GET /api/categories': aCategoryList([aCategory({ id: 'chess', name: 'Chess' })]),
      'GET /api/categories/chess/videos': [],
      'GET /api/preferences': aPreferences({ categories_enabled: false }),
    })

    expect(await screen.findByText('Categories are turned off.')).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'Preferences' })).toHaveAttribute('href', '/preferences')
    expect(screen.queryByRole('heading', { name: 'Chess' })).not.toBeInTheDocument()
  })

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
    await screen.findByText('Newest')
    const cards = screen.getAllByRole('listitem')
    expect(cards.map((card) => within(card).getAllByRole('link')[1]?.textContent)).toEqual([
      'Newest',
      'Older',
    ])
  })

  it('tints the header with the soft tint of the category color and leaves the cards untinted', async () => {
    renderCategory({
      'GET /api/categories': aCategoryList([
        aCategory({ id: 'chess', name: 'Chess', color: '#0ea5e9', channels: ['@hikaru'] }),
      ]),
      'GET /api/categories/chess/videos': [
        aHomeVideo({ title: 'Opening', category_ids: ['chess'] }),
      ],
    })

    const heading = await screen.findByRole('heading', { name: 'Chess' })
    const card = await screen.findByRole('listitem')

    expect(heading.closest('header')).toHaveStyle({ background: softTint('#0ea5e9') })
    expect(card.style.background).toBe('')
  })

  it('shows progress and time left on an in-progress card', async () => {
    renderCategory({
      'GET /api/categories': aCategoryList([
        aCategory({ id: 'chess', name: 'Chess', channels: ['@hikaru'] }),
      ]),
      'GET /api/channels': [aChannel({ id: '@hikaru', name: 'Hikaru' })],
      'GET /api/categories/chess/videos': [
        aHomeVideo({ title: 'Fresh', duration_seconds: 120 }),
        aHomeVideo({ title: 'Started', position_seconds: 120, duration_seconds: 600 }),
      ],
    })

    await screen.findByText('Started')
    const [fresh, started] = screen.getAllByRole('listitem') as [HTMLElement, HTMLElement]
    expect(within(started).getByRole('progressbar', { name: 'Watch progress' })).toHaveAttribute(
      'aria-valuenow',
      '20',
    )
    expect(within(started).getByText('8m left')).toBeInTheDocument()
    expect(within(fresh).queryByRole('progressbar')).not.toBeInTheDocument()
    expect(within(fresh).getByText('2:00')).toBeInTheDocument()
  })

  it('asks before marking a category with 3 unwatched videos watched', async () => {
    const markWatched = vi.fn(() => null)
    renderCategory({
      'GET /api/categories': aCategoryList([
        aCategory({ id: 'chess', name: 'Chess', channels: ['@hikaru'], unwatched_count: 3 }),
      ]),
      'GET /api/channels': [aChannel({ id: '@hikaru', name: 'Hikaru' })],
      'GET /api/categories/chess/videos': [aHomeVideo({ id: 'v1' })],
      'POST /api/categories/chess/watched': markWatched,
    })

    const user = userEvent.setup()
    await user.click(await screen.findByRole('button', { name: 'More actions for Chess' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Mark all watched' }))

    expect(
      await screen.findByRole('dialog', { name: 'Mark 3 videos in "Chess" watched?' }),
    ).toBeInTheDocument()
    expect(markWatched).not.toHaveBeenCalled()
  })

  it('marks all watched and deletes from the actions menu', async () => {
    let watched = false
    let category = aCategory({
      id: 'chess',
      name: 'Chess',
      channels: ['@hikaru'],
      unwatched_count: 2,
    })
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
    await user.click(await screen.findByRole('button', { name: 'More actions for Chess' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Mark all watched' }))
    await user.click(await screen.findByRole('button', { name: 'Mark watched' }))

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
    const dialogTitle = within(await screen.findByRole('dialog')).getByRole('heading', {
      level: 2,
    }).textContent
    empty.unmount()
    const withoutVideos = renderCategory(routes, '/categories/science')
    const noVideosMessage = (await screen.findByText(/No videos/)).textContent
    withoutVideos.unmount()

    renderCategory(routes, '/categories/missing')

    expect(explanation).toBe(
      'Add channels and playlists to Chess to watch their videos here, newest first.',
    )
    expect(items).toEqual(['Change color', 'Manage sources', 'Delete'])
    expect(dialogTitle).toBe('Manage sources in Chess')
    expect(noVideosMessage).toBe('No videos synced yet.')
    expect(await screen.findByText('Category not found.')).toBeInTheDocument()
  })

  it('saves the color chosen through the "Change color" action and tints the header with it', async () => {
    let chess = aCategory({ id: 'chess', name: 'Chess', color: '#0ea5e9', channels: ['@hikaru'] })
    const recolor = vi.fn(() => {
      chess = { ...chess, color: '#8b5cf6' }
      return null
    })
    const { fetchMock } = renderCategory({
      'GET /api/categories': () => aCategoryList([chess]),
      'GET /api/categories/chess/videos': [],
      'PATCH /api/categories/chess': recolor,
    })
    const user = userEvent.setup()

    await user.click(await screen.findByRole('button', { name: 'More actions for Chess' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Change color' }))
    await user.click(await screen.findByRole('radio', { name: 'Violet' }))

    // The open color dialog hides the page from assistive technology.
    const header = screen.getByRole('heading', { name: 'Chess', hidden: true }).closest('header')
    await waitFor(() => expect(header).toHaveStyle({ background: softTint('#8b5cf6') }))
    await waitFor(() => expect(recolor).toHaveBeenCalledOnce())
    const [, init] = fetchMock.mock.calls.find(([url]) => url === '/api/categories/chess') as [
      string,
      RequestInit,
    ]
    expect(JSON.parse(init.body as string)).toEqual({ color: '#8b5cf6' })
  })

  it('saves a custom hue once, when the slider settles', async () => {
    let chess = aCategory({ id: 'chess', name: 'Chess', color: '#0ea5e9', channels: ['@hikaru'] })
    const recolor = vi.fn(() => {
      chess = { ...chess, color: '#04a3aa' }
      return null
    })
    const { fetchMock } = renderCategory({
      'GET /api/categories': () => aCategoryList([chess]),
      'GET /api/categories/chess/videos': [],
      'PATCH /api/categories/chess': recolor,
    })
    const user = userEvent.setup()

    await user.click(await screen.findByRole('button', { name: 'More actions for Chess' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Change color' }))
    await user.click(await screen.findByRole('radio', { name: 'Custom' }))
    const hue = screen.getByRole('slider', { name: 'Hue' })
    ;['120', '160', '187', '200'].forEach((value) => fireEvent.change(hue, { target: { value } }))

    // The open color dialog hides the page from assistive technology.
    const header = screen.getByRole('heading', { name: 'Chess', hidden: true }).closest('header')
    await waitFor(() => expect(header).toHaveStyle({ background: softTint('#04a3aa') }))
    await waitFor(() => expect(recolor).toHaveBeenCalledOnce())
    const saves = fetchMock.mock.calls.filter(([url]) => url === '/api/categories/chess') as [
      string,
      RequestInit,
    ][]
    expect(saves.map(([, init]) => JSON.parse(init.body as string))).toEqual([{ color: '#04a3aa' }])
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

  it('opens a card in the category, playing it with the category as the queue', async () => {
    const hikaru = {
      kind: 'channel' as const,
      id: '@hikaru',
      name: 'Hikaru',
      path: 'channels/hikaru',
      avatar_filename: null,
    }
    const endgames = {
      kind: 'playlist' as const,
      id: 'PLend',
      name: 'Endgames',
      path: 'playlists/endgames',
      avatar_filename: null,
    }
    const { fetchMock } = renderCategory({
      'GET /api/categories': aCategoryList([
        aCategory({ id: 'chess', name: 'Chess', channels: ['@hikaru'], playlists: ['PLend'] }),
      ]),
      'GET /api/channels': [aChannel({ id: '@hikaru', name: 'Hikaru', path: 'channels/hikaru' })],
      'GET /api/categories/chess/videos': [
        aCategoryVideo({ id: 'blitz', title: 'Blitz', filename: 'blitz.mp4', source: hikaru }),
        aCategoryVideo({
          id: 'rook',
          title: 'Rook endings',
          filename: 'rook.mp4',
          source: endgames,
        }),
      ],
    })
    const user = userEvent.setup()

    await user.click(await screen.findByRole('link', { name: 'Rook endings' }))
    await waitFor(() =>
      expect(document.querySelector('video')?.getAttribute('src')).toBe(
        '/media/playlists/endgames/rook.mp4',
      ),
    )
    const route = screen.getByRole('status', { name: 'Current route' }).textContent
    const autoplay = document.querySelector('video')?.hasAttribute('autoplay')
    const queue = screen.getAllByRole('button', { name: /^2:00 / })
    await user.click(screen.getByRole('button', { name: '2:00 Blitz' }))

    await waitFor(() =>
      expect(document.querySelector('video')?.getAttribute('src')).toBe(
        '/media/channels/hikaru/blitz.mp4',
      ),
    )
    expect(route).toBe('/categories/chess?video=rook')
    expect(autoplay).toBe(true)
    expect(queue.map((row) => row.textContent)).toEqual(['2:00Blitz', '2:00Rook endings'])
    expect(screen.getByRole('link', { name: 'Hikaru' })).toHaveAttribute(
      'href',
      '/channels/@hikaru',
    )
    // The category's own list feeds the player: no source's video list is fetched.
    expect(
      fetchMock.mock.calls.map(([url]) => url as string).filter((url) => url.includes('/videos')),
    ).not.toContainEqual(expect.stringMatching(/^\/api\/(channels|playlists)\//))
  })

  it('shows the header from the categories list at once, with placeholders while the videos load', async () => {
    renderCategory({
      'GET /api/categories': aCategoryList([
        aCategory({ id: 'chess', name: 'Chess', color: '#0ea5e9', channels: ['@hikaru'] }),
      ]),
      'GET /api/categories/chess/videos': pendingForever(),
    })

    const heading = await screen.findByRole('heading', { name: 'Chess' })
    const header = heading.closest('header') as HTMLElement
    expect(header).toHaveStyle({ background: softTint('#0ea5e9') })
    expect(
      within(header).getByRole('button', { name: 'More actions for Chess' }),
    ).toBeInTheDocument()
    expect(header.querySelector('[data-slot="skeleton"]')).not.toBeNull()
    expect(within(header).queryByText(/sources/)).not.toBeInTheDocument()
    const status = screen.getByRole('status', { name: '' })
    expect(status).toHaveTextContent('Loading…')
    expect(status.closest('[aria-busy="true"]')).not.toBeNull()
  })
})
