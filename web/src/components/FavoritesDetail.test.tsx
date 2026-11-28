import { describe, expect, it, vi } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Route, Routes } from 'react-router-dom'
import { FavoritesDetail } from './FavoritesDetail'
import {
  aCategoryVideo,
  aPreferences,
  mockApi,
  renderWithProviders,
  type Routes as ApiRoutes,
} from '@/test/helpers'

function renderFavorites(routes: ApiRoutes) {
  mockApi({
    'GET /api/channels': [],
    'GET /api/playlists': [],
    'GET /api/preferences': aPreferences(),
    ...routes,
  })
  return renderWithProviders(
    <Routes>
      <Route path="/favorites" element={<FavoritesDetail />} />
    </Routes>,
    { route: '/favorites' },
  )
}

const TALKS = {
  kind: 'channel' as const,
  id: '@talks',
  name: 'Talks',
  path: 'channels/talks',
  avatar_filename: null,
}

const LECTURES = {
  kind: 'playlist' as const,
  id: 'PLlectures',
  name: 'Lectures',
  path: 'playlists/lectures',
  avatar_filename: null,
}

/** The titles of the cards shown, in order. */
function cardTitles(): string[] {
  return within(screen.getByRole('list'))
    .getAllByRole('listitem')
    .map((card) => within(card).getAllByRole('link')[0]?.textContent ?? '')
}

describe('FavoritesDetail', () => {
  it('lists favorites in server order with their source', async () => {
    renderFavorites({
      'GET /api/favorites/videos': [
        aCategoryVideo({ title: 'Newest favorite', favorite: true, source: TALKS }),
        aCategoryVideo({ title: 'Older favorite', favorite: true, source: LECTURES }),
      ],
    })

    expect(await screen.findByRole('heading', { name: 'Favorites' })).toBeInTheDocument()
    expect(await screen.findByText('Newest favorite')).toBeInTheDocument()
    expect(cardTitles()).toEqual(['Newest favorite', 'Older favorite'])
    expect(
      within(screen.getByText('Newest favorite').closest('li') as HTMLElement).getByRole('link', {
        name: 'Talks',
      }),
    ).toBeInTheDocument()
    expect(
      within(screen.getByText('Older favorite').closest('li') as HTMLElement).getByRole('link', {
        name: 'Lectures',
      }),
    ).toBeInTheDocument()
  })

  it('says No favorites yet', async () => {
    renderFavorites({ 'GET /api/favorites/videos': [] })

    expect(await screen.findByRole('heading', { name: 'No favorites yet' })).toBeInTheDocument()
    expect(
      screen.getByText('Press the star on a video to add it to your favorites.'),
    ).toBeInTheDocument()
  })

  it('drops a video once unfavorited', async () => {
    const leaving = aCategoryVideo({ id: 'v1', title: 'Leaving', favorite: true, source: TALKS })
    const staying = aCategoryVideo({ id: 'v2', title: 'Staying', favorite: true, source: TALKS })
    const unfavorite = vi.fn(() => null)
    const favorites = vi
      .fn()
      .mockReturnValueOnce([leaving, staying])
      .mockReturnValue([staying])
    renderFavorites({
      'GET /api/favorites/videos': favorites,
      'GET /api/videos/home': { continue_watching: [], categories: [], quick_watches: [], latest: [] },
      'GET /api/categories': { categories: [] },
      'DELETE /api/videos/v1/favorite': unfavorite,
    })
    const user = userEvent.setup()

    await user.click(await screen.findByRole('button', { name: 'Actions for Leaving' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Remove from favorites' }))

    expect(unfavorite).toHaveBeenCalledOnce()
    await waitFor(() => expect(screen.queryByText('Leaving')).not.toBeInTheDocument())
    expect(cardTitles()).toEqual(['Staying'])
  })
})
