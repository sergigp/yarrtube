import { describe, expect, it, vi } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import type { InitialEntry } from 'react-router-dom'
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
import { RouteProbe } from '@/test/RouteProbe'

function renderView(routes: Routes, { route = '/categories' }: { route?: InitialEntry } = {}) {
  const fetchMock = mockApi({ 'GET /api/channels': [], 'GET /api/playlists': [], ...routes })
  return {
    fetchMock,
    ...renderWithProviders(
      <>
        <CategoriesView />
        <RouteProbe />
      </>,
      { route },
    ),
  }
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
    await screen.findByRole('region', { name: 'In Board games' })
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

  it('filters the picker and adds the checked sources', async () => {
    let chess = aCategory({ id: 'chess', name: 'Chess', channels: ['@hikaru'] })
    const { fetchMock } = renderView(
      {
        'GET /api/categories': () => aCategoryList([chess]),
        'GET /api/channels': [
          aChannel({ id: '@hikaru', name: 'Hikaru' }),
          aChannel({ id: '@veritasium', name: 'Veritasium' }),
          aChannel({ id: '@everyday', name: 'Everyday Astronaut' }),
          aChannel({ id: '@babish', name: 'Babish' }),
        ],
        'GET /api/playlists': [
          aPlaylist({ id: 'PLuniverse', name: 'Universe' }),
          aPlaylist({ id: 'PLcooking', name: 'Cooking' }),
        ],
        'PUT /api/categories/chess/members': () => {
          chess = { ...chess, channels: ['@hikaru', '@veritasium'], playlists: ['PLuniverse'] }
          return null
        },
      },
      {
        route: { pathname: '/categories', search: '?selected=chess', state: { focusAddMembers: true } },
      },
    )
    const user = userEvent.setup()
    const search = await screen.findByRole('searchbox', {
      name: 'Search channels and playlists to add',
    })
    await screen.findByRole('checkbox', { name: 'Veritasium' })
    const focused = document.activeElement === search
    const picker = screen.getByRole('group', { name: 'Add to Chess' })

    await user.type(search, 'ver')
    const matches = within(picker)
      .getAllByRole('checkbox')
      .map((checkbox) => (checkbox.closest('label')?.textContent ?? '').trim())
    await user.click(within(picker).getByRole('checkbox', { name: 'Veritasium' }))
    await user.click(within(picker).getByRole('checkbox', { name: 'Universe' }))
    await user.click(screen.getByRole('button', { name: 'Add selected' }))

    await waitFor(() =>
      expect(within(picker).queryByRole('checkbox', { name: 'Veritasium' })).not.toBeInTheDocument(),
    )
    expect(focused).toBe(true)
    expect(matches).toEqual(['Everyday Astronaut', 'Veritasium', 'Universe'])
    expect(within(picker).getAllByRole('checkbox')).toHaveLength(1)
    const [, init] = fetchMock.mock.calls.find(
      ([, init]) => (init as RequestInit | undefined)?.method === 'PUT',
    ) as [string, RequestInit]
    expect(JSON.parse(init.body as string)).toEqual({
      channels: ['@hikaru', '@veritasium'],
      playlists: ['PLuniverse'],
    })
  })

  it('deletes the selected category after confirmation', async () => {
    let categories = [
      aCategory({ id: 'chess', name: 'Chess', channels: ['@hikaru'], playlists: ['PLend'] }),
      aCategory({ id: 'science', name: 'Science' }),
    ]
    const remove = vi.fn(() => {
      categories = categories.filter((category) => category.id !== 'chess')
      return null
    })
    renderView(
      {
        'GET /api/categories': () => aCategoryList(categories),
        'DELETE /api/categories/chess': remove,
      },
      { route: '/categories?selected=chess' },
    )
    const user = userEvent.setup()

    await user.click(await screen.findByRole('button', { name: 'Delete category' }))
    const dialog = await screen.findByRole('dialog', { name: 'Delete "Chess"?' })
    const description = within(dialog).getByText(/holds/).textContent
    await user.click(within(dialog).getByRole('button', { name: 'Delete' }))

    await waitFor(() => expect(remove).toHaveBeenCalledOnce())
    expect(description).toBe(
      '"Chess" holds 2 channels and playlists. None of them, nor any video, will be deleted.',
    )
    expect(
      await within(screen.getByRole('list', { name: 'Category list' })).findAllByRole('link'),
    ).toHaveLength(1)
    expect(screen.getByRole('status', { name: 'Current route' })).toHaveTextContent(
      /^\/categories$/,
    )
    expect(screen.queryByRole('textbox', { name: 'Category name' })).not.toBeInTheDocument()
  })

  it('shows list or details on a narrow viewport', async () => {
    // The matchMedia stub reports a viewport below the desktop breakpoint.
    renderView({
      'GET /api/categories': aCategoryList([
        aCategory({ id: 'chess', name: 'Chess' }),
        aCategory({ id: 'science', name: 'Science' }),
      ]),
    })
    const user = userEvent.setup()

    await user.click(await screen.findByRole('link', { name: /^Chess/ }))
    const listWithDetails = screen.queryByRole('list', { name: 'Category list' })
    const name = screen.getByRole('textbox', { name: 'Category name' })
    await user.click(screen.getByRole('link', { name: 'Back to categories' }))

    expect(listWithDetails).not.toBeInTheDocument()
    expect(name).toHaveValue('Chess')
    expect(screen.getByRole('list', { name: 'Category list' })).toBeInTheDocument()
    expect(screen.queryByRole('textbox', { name: 'Category name' })).not.toBeInTheDocument()
  })

  it('invites to create a category when there is none', async () => {
    renderView({ 'GET /api/categories': aCategoryList([]) })

    expect(
      await screen.findByText('No categories yet. Create one to group channels and playlists.'),
    ).toBeInTheDocument()
    await userEvent.setup().click(screen.getByRole('button', { name: 'New category' }))
    expect(await screen.findByRole('dialog', { name: 'Add category' })).toBeInTheDocument()
  })

  it("links to the selected category's view", async () => {
    renderView(
      { 'GET /api/categories': aCategoryList([aCategory({ id: 'chess', name: 'Chess' })]) },
      { route: '/categories?selected=chess' },
    )

    expect(await screen.findByRole('link', { name: 'Open Chess' })).toHaveAttribute(
      'href',
      '/categories/chess',
    )
  })
})
