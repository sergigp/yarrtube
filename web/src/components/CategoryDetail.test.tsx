import { describe, expect, it } from 'vitest'
import { screen, within } from '@testing-library/react'
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
})
