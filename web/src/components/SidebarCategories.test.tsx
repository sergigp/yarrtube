import { beforeEach, describe, expect, it } from 'vitest'
import { screen, within } from '@testing-library/react'
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
})
