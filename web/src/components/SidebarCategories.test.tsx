import { beforeEach, describe, expect, it, vi } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Route, Routes as RouterRoutes } from 'react-router-dom'
import { CategoryDetail } from './CategoryDetail'
import { Sidebar } from './Sidebar'
import {
  aCategory,
  aCategoryList,
  aChannel,
  mockApi,
  renderWithProviders,
  type Routes as ApiRoutes,
} from '@/test/helpers'
import { RouteProbe } from '@/test/RouteProbe'

/** Renders the sidebar beside a probe of the current route and, optionally, the category view. */
function renderSidebar(routes: ApiRoutes, { route = '/', withCategoryView = false } = {}) {
  mockApi({ 'GET /api/channels': [], 'GET /api/playlists': [], ...routes })
  return renderWithProviders(
    <>
      {withCategoryView && (
        <RouterRoutes>
          <Route path="/categories/:id" element={<CategoryDetail />} />
        </RouterRoutes>
      )}
      <Sidebar
        open
        onClose={() => {}}
        onAddChannel={() => {}}
        onAddPlaylist={() => {}}
        onAddCategory={() => {}}
      />
      <RouteProbe />
    </>,
    { route },
  )
}

function categoryRowNames(): string[] {
  return within(screen.getByRole('list', { name: 'Categories' }))
    .getAllByRole('link')
    .map((link) => link.textContent ?? '')
}

describe('SidebarCategories', () => {
  beforeEach(() => {
    window.localStorage.clear()
  })

  it('lists every category above channels with badges, in unread order', async () => {
    renderSidebar({
      'GET /api/channels': [aChannel({ name: 'Chan A' })],
      'GET /api/categories': aCategoryList([
        aCategory({ name: 'Alpha', unwatched_count: 0 }),
        aCategory({ name: 'Bravo', unwatched_count: 4 }),
        aCategory({ name: 'Charlie', unwatched_count: 12 }),
        aCategory({ name: 'Delta', unwatched_count: 0 }),
      ]),
    })

    await screen.findByText('Charlie')
    const categoriesHeading = screen.getByRole('button', { name: 'Categories' })
    const channelsHeading = screen.getByRole('button', { name: 'Channels' })

    expect(categoryRowNames()).toEqual(['Charlie12', 'Bravo4', 'Alpha', 'Delta'])
    expect(
      categoriesHeading.compareDocumentPosition(channelsHeading) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy()
    expect(screen.getByTitle('12 unwatched')).toHaveTextContent('12')
  })

  it('opens a category and marks its row active', async () => {
    renderSidebar({
      'GET /api/categories': aCategoryList([
        aCategory({ id: 'chess', name: 'Chess' }),
        aCategory({ id: 'science', name: 'Science' }),
      ]),
    })

    await userEvent.setup().click(await screen.findByRole('link', { name: 'Chess' }))

    expect(screen.getByRole('link', { name: 'Chess' })).toHaveAttribute('aria-current', 'page')
    expect(screen.getByRole('link', { name: 'Science' })).not.toHaveAttribute('aria-current')
  })

  it('folds with the unwatched total badge and marks the heading active on a category route', async () => {
    renderSidebar(
      {
        'GET /api/categories': aCategoryList(
          [
            aCategory({ id: 'chess', name: 'Chess', unwatched_count: 3 }),
            aCategory({ id: 'science', name: 'Science', unwatched_count: 14 }),
          ],
          { unwatched_total: 16 },
        ),
      },
      { route: '/categories/chess' },
    )

    await screen.findByText('Chess')
    const addControlWhileUnfolded = screen.queryByRole('button', { name: 'Add category' }) !== null
    const heading = screen.getByRole('button', { name: 'Categories' })
    await userEvent.setup().click(heading)

    expect(addControlWhileUnfolded).toBe(true)
    expect(heading).toHaveAttribute('aria-expanded', 'false')
    expect(heading).toHaveAttribute('aria-current', 'true')
    expect(within(heading).getByLabelText('16 unwatched')).toHaveTextContent('16')
    expect(screen.queryByText('Chess')).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Add category' })).not.toBeInTheDocument()
  })

  it('manages sources and deletes from the row menu, with the member-count confirmation', async () => {
    let categories = [
      aCategory({
        id: 'chess',
        name: 'Chess',
        channels: ['@GothamChess', '@Hikaru', '@ChessBrah'],
        playlists: ['PLendgames'],
      }),
      aCategory({ id: 'science', name: 'Science' }),
    ]
    const remove = vi.fn(() => {
      categories = categories.filter((category) => category.id !== 'chess')
      return null
    })
    renderSidebar(
      {
        'GET /api/categories': () => aCategoryList(categories),
        'DELETE /api/categories/chess': remove,
        'GET /api/categories/chess/videos': [],
        'GET /api/categories/science/videos': [],
      },
      { route: '/categories/chess', withCategoryView: true },
    )
    const user = userEvent.setup()

    await user.click(await screen.findByRole('button', { name: 'Actions for Chess' }))
    const items = (await screen.findAllByRole('menuitem')).map((item) => item.textContent)
    await user.click(screen.getByRole('menuitem', { name: 'Delete' }))
    const dialog = await screen.findByRole('dialog', { name: 'Delete "Chess"?' })
    const description = within(dialog).getByText(/holds/).textContent
    await user.click(within(dialog).getByRole('button', { name: 'Delete' }))
    await waitFor(() => expect(screen.queryByText('Chess')).not.toBeInTheDocument())
    await waitFor(() =>
      expect(screen.getByRole('status', { name: 'Current route' })).toHaveTextContent(/^\/$/),
    )
    const routeAfterDelete = screen.getByRole('status', { name: 'Current route' }).textContent
    await user.click(screen.getByRole('button', { name: 'Actions for Science' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Manage sources' }))

    expect(items).toEqual(['Manage sources', 'Delete'])
    expect(description).toBe(
      '"Chess" holds 4 channels and playlists. None of them, nor any video, will be deleted.',
    )
    expect(remove).toHaveBeenCalledOnce()
    expect(routeAfterDelete).toBe('/')
    expect(await screen.findByRole('dialog', { name: 'Manage sources in Science' })).toBeVisible()
    expect(
      screen.getByRole('status', { name: 'Current route', hidden: true }),
    ).toHaveTextContent('/categories/science')
  })

  it('is not filtered by the sidebar search', async () => {
    renderSidebar({
      'GET /api/channels': Array.from({ length: 16 }, (_, index) =>
        aChannel({ name: `Chan ${index}` }),
      ),
      'GET /api/categories': aCategoryList([
        aCategory({ name: 'Chess' }),
        aCategory({ name: 'Science' }),
      ]),
    })

    const search = await screen.findByRole('searchbox', { name: 'Search channels and playlists' })
    await screen.findByText('Chess')
    await userEvent.setup().type(search, 'Chan 1')

    expect(categoryRowNames()).toEqual(['Chess', 'Science'])
  })
})
