import { useState } from 'react'
import { describe, expect, it, vi } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { SearchDialog } from './SearchDialog'
import { RouteProbe } from '@/test/RouteProbe'
import {
  aCategory,
  aCategoryList,
  aChannel,
  aHomeVideo,
  aPlaylist,
  aPreferences,
  mockApi,
  pendingForever,
  renderWithProviders,
  type Routes,
} from '@/test/helpers'

const BETTATECH = {
  kind: 'channel' as const,
  id: '@bettatech',
  name: 'BettaTech',
  path: 'channels/bettatech',
  avatar_filename: null,
}

/** The dialog as the shell holds it: open until it asks to close. */
function OpenSearch() {
  const [open, setOpen] = useState(true)
  return (
    <>
      <SearchDialog open={open} onOpenChange={setOpen} />
      <RouteProbe />
    </>
  )
}

function renderSearch(routes: Routes) {
  const fetchMock = mockApi({
    'GET /api/categories': aCategoryList([aCategory({ id: 'chess', name: 'Chess' })]),
    'GET /api/channels': [aChannel({ id: '@andras', name: 'ChessCoach Andras' })],
    'GET /api/playlists': [aPlaylist({ id: 'PLmusic', name: 'Music' })],
    'GET /api/preferences': aPreferences(),
    ...routes,
  })
  renderWithProviders(<OpenSearch />)
  return { fetchMock, user: userEvent.setup() }
}

/** The names listed under the group headed `name`, or null when it isn't shown. */
function groupOptions(name: string): string[] | null {
  const group = screen.queryByRole('group', { name })
  return (
    group &&
    within(group)
      .getAllByRole('option')
      .map((option) => option.textContent ?? '')
  )
}

function videoSearches(fetchMock: ReturnType<typeof vi.fn>): string[] {
  return fetchMock.mock.calls
    .map(([url]) => String(url))
    .filter((url) => url.startsWith('/api/videos/search'))
}

describe('SearchDialog', () => {
  it('opens with its field focused', () => {
    renderSearch({})

    expect(screen.getByRole('combobox', { name: 'Search' })).toHaveFocus()
  })

  it('groups categories, channels and videos matching the text, hiding empty groups', async () => {
    const { user } = renderSearch({
      'GET /api/videos/search?q=chess': {
        videos: [aHomeVideo({ title: 'Chess Tactics: Spot the Trigger', source: BETTATECH })],
      },
    })

    await user.type(screen.getByRole('combobox', { name: 'Search' }), 'chess')

    await waitFor(() => expect(groupOptions('Videos')).not.toBeNull())
    expect(groupOptions('Categories')).toEqual(['Chess'])
    expect(groupOptions('Channels')).toEqual(['ChessCoach Andras'])
    expect(groupOptions('Playlists')).toBeNull()
    expect(groupOptions('Videos')).toEqual(['Chess Tactics: Spot the TriggerBettaTech'])
    expect(screen.getAllByRole('group').map((group) => group.getAttribute('aria-label'))).toEqual([
      'Categories',
      'Channels',
      'Videos',
    ])
  })

  it('searches only names for a single character', async () => {
    const { fetchMock, user } = renderSearch({})

    await user.type(screen.getByRole('combobox', { name: 'Search' }), 'm')
    await new Promise((resolve) => setTimeout(resolve, 400))

    expect(groupOptions('Playlists')).toEqual(['Music'])
    expect(videoSearches(fetchMock)).toEqual([])
  })

  it('searches videos once, after the user pauses typing', async () => {
    const { fetchMock, user } = renderSearch({
      'GET /api/videos/search?q=chess': { videos: [] },
    })

    await user.type(screen.getByRole('combobox', { name: 'Search' }), 'chess')

    await waitFor(() => expect(videoSearches(fetchMock)).toEqual(['/api/videos/search?q=chess']))
  })

  it('opens a chosen video in its channel view and closes', async () => {
    const { user } = renderSearch({
      'GET /api/videos/search?q=rust': {
        videos: [aHomeVideo({ id: 'vid1', title: 'Rust in 100 seconds', source: BETTATECH })],
      },
    })

    await user.type(screen.getByRole('combobox', { name: 'Search' }), 'rust')
    await user.click(await screen.findByRole('option', { name: /Rust in 100 seconds/ }))

    expect(screen.getByRole('status', { name: 'Current route' })).toHaveTextContent(
      '/channels/@bettatech?video=vid1',
    )
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
  })

  it('moves through the results with the arrow keys and chooses with Enter', async () => {
    const { user } = renderSearch({ 'GET /api/videos/search?q=chess': { videos: [] } })
    const field = screen.getByRole('combobox', { name: 'Search' })

    await user.type(field, 'chess')
    await screen.findByRole('option', { name: 'ChessCoach Andras' })
    await user.keyboard('{ArrowDown}')

    expect(field).toHaveAttribute(
      'aria-activedescendant',
      screen.getByRole('option', { name: 'ChessCoach Andras' }).id,
    )
    await user.keyboard('{Enter}')
    expect(screen.getByRole('status', { name: 'Current route' })).toHaveTextContent(
      '/channels/@andras',
    )
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
  })

  it('opens a chosen category in its view', async () => {
    const { user } = renderSearch({ 'GET /api/videos/search?q=ch': { videos: [] } })

    await user.type(screen.getByRole('combobox', { name: 'Search' }), 'ch')
    await user.click(screen.getByRole('option', { name: 'Chess' }))

    expect(screen.getByRole('status', { name: 'Current route' })).toHaveTextContent(
      '/categories/chess',
    )
  })

  it('says nothing matches the text', async () => {
    const { user } = renderSearch({ 'GET /api/videos/search?q=zzz': { videos: [] } })

    await user.type(screen.getByRole('combobox', { name: 'Search' }), 'zzz')

    expect(await screen.findByText('Nothing matches "zzz".')).toBeInTheDocument()
  })

  it('shows that videos are being searched', async () => {
    const { user } = renderSearch({ 'GET /api/videos/search?q=chess': pendingForever() })

    await user.type(screen.getByRole('combobox', { name: 'Search' }), 'chess')

    const videos = await screen.findByRole('group', { name: 'Videos' })
    expect(videos).toHaveTextContent('Searching…')
  })

  it('says videos could not be searched, still showing the name matches', async () => {
    const { user } = renderSearch({
      'GET /api/videos/search?q=chess': { status: 500, error: 'boom' },
    })

    await user.type(screen.getByRole('combobox', { name: 'Search' }), 'chess')

    expect(await screen.findByText('Videos could not be searched.')).toBeInTheDocument()
    expect(groupOptions('Categories')).toEqual(['Chess'])
  })

  it('closes on Escape', async () => {
    const { user } = renderSearch({})

    await user.keyboard('{Escape}')

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
  })

  it('searches no categories while categories are turned off', async () => {
    const { user } = renderSearch({
      'GET /api/preferences': aPreferences({ categories_enabled: false }),
      'GET /api/videos/search?q=chess': { videos: [] },
    })

    await user.type(screen.getByRole('combobox', { name: 'Search' }), 'chess')

    expect(await screen.findByRole('option', { name: 'ChessCoach Andras' })).toBeInTheDocument()
    expect(groupOptions('Categories')).toBeNull()
  })
})
