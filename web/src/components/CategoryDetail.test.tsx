import { describe, expect, it, vi } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Route, Routes } from 'react-router-dom'
import { CategoryDetail } from './CategoryDetail'
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

function renderCategory(routes: ApiRoutes, route = '/categories/chess') {
  const fetchMock = mockApi({ 'GET /api/channels': [], 'GET /api/playlists': [], ...routes })
  renderWithProviders(
    <Routes>
      <Route path="/categories/:id" element={<CategoryDetail />} />
    </Routes>,
    { route },
  )
  return { fetchMock }
}

describe('CategoryDetail', () => {
  it('shows the members and video cards newest first', async () => {
    renderCategory({
      'GET /api/categories': aCategoryList([
        aCategory({
          id: 'chess',
          name: 'Chess',
          channels: ['@hikaru', '@gotham'],
          playlists: ['PLend'],
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
    expect(await screen.findByText('GothamChess · Hikaru · Endgame course')).toBeInTheDocument()
    const cards = await screen.findAllByRole('listitem')
    expect(cards.map((card) => within(card).getAllByRole('link')[1]?.textContent)).toEqual([
      'Newest',
      'Older',
    ])
  })

  it('marks all watched', async () => {
    let watched = false
    let category = aCategory({ id: 'chess', name: 'Chess', channels: ['@hikaru'], unwatched_count: 2 })
    const markWatched = vi.fn(() => {
      watched = true
      category = { ...category, unwatched_count: 0 }
      return null
    })
    const { fetchMock } = renderCategory({
      'GET /api/categories': () => aCategoryList([category]),
      'GET /api/channels': [aChannel({ id: '@hikaru', name: 'Hikaru' })],
      'GET /api/categories/chess/videos': () => [
        aHomeVideo({ id: 'v1', watched }),
        aHomeVideo({ id: 'v2', watched }),
      ],
      'POST /api/categories/chess/watched': markWatched,
    })

    await screen.findAllByRole('listitem')
    await userEvent.setup().click(screen.getByRole('button', { name: 'Mark all watched' }))

    expect(await screen.findAllByRole('img', { name: 'Watched' })).toHaveLength(2)
    expect(markWatched).toHaveBeenCalledOnce()
    await waitFor(() =>
      expect(
        fetchMock.mock.calls.filter(([url]) => url === '/api/categories').length,
      ).toBeGreaterThan(1),
    )
  })
})
