import { describe, expect, it } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { CategoriesView } from './CategoriesView'
import {
  aCategory,
  aCategoryList,
  aChannel,
  aPlaylist,
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

  it('renames in place, and rejects a taken name keeping the saved one', async () => {
    let chess = aCategory({ id: 'chess', name: 'Chess' })
    const science = aCategory({ id: 'science', name: 'Science' })
    const responses = [
      () => {
        chess = { ...chess, name: 'Board games' }
        return null
      },
      () => ({ status: 409, error: 'a category named "Science" already exists' }),
    ]
    const { fetchMock } = renderView(
      {
        'GET /api/categories': () => aCategoryList([chess, science]),
        'PATCH /api/categories/chess': () => responses.shift()?.(),
      },
      { route: '/categories?selected=chess' },
    )
    const user = userEvent.setup()
    const name = await screen.findByRole('textbox', { name: 'Category name' })

    await user.clear(name)
    await user.type(name, 'Board games')
    await user.tab()
    const list = screen.getByRole('list', { name: 'Category list' })
    await within(list).findByText('Board games')
    await user.clear(name)
    await user.type(name, 'Science{Enter}')

    expect(
      await screen.findByText('a category named "Science" already exists'),
    ).toBeInTheDocument()
    expect(name).toHaveValue('Board games')
    expect(
      fetchMock.mock.calls
        .filter(([, init]) => (init as RequestInit | undefined)?.method === 'PATCH')
        .map(([, init]) => JSON.parse((init as RequestInit).body as string)),
    ).toEqual([{ name: 'Board games' }, { name: 'Science' }])
  })

  it('groups members by kind and removes one', async () => {
    let chess = aCategory({
      id: 'chess',
      name: 'Chess',
      channels: ['@hikaru', '@gotham'],
      playlists: ['PLend'],
    })
    const { fetchMock } = renderView(
      {
        'GET /api/categories': () => aCategoryList([chess]),
        'GET /api/channels': [
          aChannel({ id: '@hikaru', name: 'Hikaru' }),
          aChannel({ id: '@gotham', name: 'GothamChess' }),
        ],
        'GET /api/playlists': [aPlaylist({ id: 'PLend', name: 'Endgame course' })],
        'PUT /api/categories/chess/members': () => {
          chess = { ...chess, channels: ['@gotham'] }
          return null
        },
      },
      { route: '/categories?selected=chess' },
    )
    const members = await screen.findByRole('region', { name: 'In Chess' })
    const namesOf = (kind: string) =>
      within(within(members).getByRole('list', { name: kind }))
        .queryAllByRole('listitem')
        .map((item) => item.firstChild?.textContent)
    await within(members).findByText('Hikaru')
    const before = [namesOf('Channels'), namesOf('Playlists')]

    await userEvent.setup().click(within(members).getByRole('button', { name: 'Remove Hikaru' }))

    await waitFor(() => expect(within(members).queryByText('Hikaru')).not.toBeInTheDocument())
    expect(before).toEqual([['GothamChess', 'Hikaru'], ['Endgame course']])
    const [, init] = fetchMock.mock.calls.find(
      ([, init]) => (init as RequestInit | undefined)?.method === 'PUT',
    ) as [string, RequestInit]
    expect(JSON.parse(init.body as string)).toEqual({ channels: ['@gotham'], playlists: ['PLend'] })
  })
})
