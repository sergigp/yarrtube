import { beforeEach, describe, expect, it, vi } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { useLocation } from 'react-router-dom'
import { Sidebar } from './Sidebar'
import {
  aCategory,
  aCategoryList,
  aChannel,
  mockApi,
  renderWithProviders,
  type Routes,
} from '@/test/helpers'

/** Renders the sidebar beside stand-in views that name the route shown. */
function renderSidebar(routes: Routes, { route = '/' } = {}) {
  mockApi({ 'GET /api/channels': [], 'GET /api/playlists': [], ...routes })
  return renderWithProviders(
    <>
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

function RouteProbe() {
  const location = useLocation()
  return <output aria-label="Current route">{`${location.pathname}${location.search}`}</output>
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

  it('manage control navigates without folding', async () => {
    renderSidebar({ 'GET /api/categories': aCategoryList([aCategory({ name: 'Chess' })]) })

    await screen.findByText('Chess')
    await userEvent.setup().click(screen.getByRole('link', { name: 'Manage categories' }))

    expect(screen.getByRole('status', { name: 'Current route' })).toHaveTextContent(
      /^\/categories$/,
    )
    expect(screen.getByRole('button', { name: 'Categories' })).toHaveAttribute(
      'aria-expanded',
      'true',
    )
    expect(screen.getByText('Chess')).toBeInTheDocument()
  })

  it('folds with the muted unwatched total and marks the heading active on a category route', async () => {
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
    const addControlWhileUnfolded =
      screen.queryByRole('button', { name: 'Add category' })?.textContent
    const heading = screen.getByRole('button', { name: 'Categories' })
    await userEvent.setup().click(heading)

    expect(addControlWhileUnfolded).toBe('Add category')
    expect(heading).toHaveAttribute('aria-expanded', 'false')
    expect(heading).toHaveAttribute('aria-current', 'true')
    expect(within(heading).getByLabelText('16 unwatched')).toHaveTextContent('16')
    expect(screen.queryByText('Chess')).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Add category' })).not.toBeInTheDocument()
  })

  it('manages and deletes from the row menu, with the member-count confirmation', async () => {
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
      },
      { route: '/categories/chess' },
    )
    const user = userEvent.setup()

    await user.click(await screen.findByRole('button', { name: 'Actions for Chess' }))
    const items = (await screen.findAllByRole('menuitem')).map((item) => item.textContent)
    await user.click(screen.getByRole('menuitem', { name: 'Delete' }))
    const dialog = await screen.findByRole('dialog', { name: 'Delete "Chess"?' })
    const description = within(dialog).getByText(/holds/).textContent
    await user.click(within(dialog).getByRole('button', { name: 'Delete' }))
    await waitFor(() => expect(screen.queryByText('Chess')).not.toBeInTheDocument())
    const routeAfterDelete = screen.getByRole('status', { name: 'Current route' }).textContent
    await user.click(screen.getByRole('button', { name: 'Actions for Science' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Manage' }))

    expect(items).toEqual(['Manage', 'Delete'])
    expect(description).toBe(
      '"Chess" holds 4 channels and playlists. None of them, nor any video, will be deleted.',
    )
    expect(remove).toHaveBeenCalledOnce()
    expect(routeAfterDelete).toBe('/')
    expect(screen.getByRole('status', { name: 'Current route' })).toHaveTextContent(
      '/categories?selected=science',
    )
  })
})
