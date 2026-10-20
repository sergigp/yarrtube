import { describe, expect, it } from 'vitest'
import { screen, within } from '@testing-library/react'
import { CategoriesView } from './CategoriesView'
import {
  aCategory,
  aCategoryList,
  mockApi,
  renderWithProviders,
  type Routes,
} from '@/test/helpers'

function renderView(routes: Routes, { route = '/categories' } = {}) {
  const fetchMock = mockApi({ 'GET /api/channels': [], 'GET /api/playlists': [], ...routes })
  return { fetchMock, ...renderWithProviders(<CategoriesView />, { route }) }
}

describe('CategoriesView', () => {
  it('lists categories with member counts and selects one from the address', async () => {
    const routes = {
      'GET /api/categories': aCategoryList([
        aCategory({ id: 'chess', name: 'Chess', channels: ['@a', '@b'], playlists: ['PL1'] }),
        aCategory({ id: 'cooking', name: 'cooking', channels: ['@c'] }),
        aCategory({ id: 'science', name: 'Science' }),
      ]),
    }
    const { unmount } = renderView(routes)
    const list = await screen.findByRole('list', { name: 'Category list' })
    const rows = within(list)
      .getAllByRole('link')
      .map((link) => link.textContent)
    unmount()

    renderView(routes, { route: '/categories?selected=chess' })

    expect(rows).toEqual(['Chess3 members', 'cooking1 member', 'Science0 members'])
    expect(await screen.findByRole('textbox', { name: 'Category name' })).toHaveValue('Chess')
  })
})
