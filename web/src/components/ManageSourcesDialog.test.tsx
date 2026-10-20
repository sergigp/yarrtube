import { describe, expect, it, vi } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import type { Category } from '@/api/types'
import { ManageSourcesDialog } from './ManageSourcesDialog'
import {
  aCategory,
  aChannel,
  aPlaylist,
  mockApi,
  renderWithProviders,
  type Routes,
} from '@/test/helpers'

function renderDialog(category: Category, routes: Routes) {
  const fetchMock = mockApi({ 'GET /api/channels': [], 'GET /api/playlists': [], ...routes })
  const onOpenChange = vi.fn()
  renderWithProviders(
    <ManageSourcesDialog category={category} open onOpenChange={onOpenChange} />,
  )
  return { fetchMock, onOpenChange }
}

/** Each chip of a group as its name and whether it's pressed. */
async function chipsOf(group: string): Promise<[string, boolean][]> {
  return within(await screen.findByRole('toolbar', { name: group }))
    .getAllByRole('button')
    .map((chip) => [chip.textContent ?? '', chip.getAttribute('aria-pressed') === 'true'])
}

/** The names of every chip shown, channels first. */
function shownChips(): string[] {
  return screen.queryAllByRole('toolbar').flatMap((group) =>
    within(group)
      .getAllByRole('button')
      .map((chip) => chip.textContent ?? ''),
  )
}

const chess = aCategory({ id: 'chess', name: 'Chess', channels: ['@hikaru'] })

const library = {
  'GET /api/channels': [
    aChannel({ id: '@hikaru', name: 'Hikaru', avatar_filename: 'hikaru.jpg' }),
    aChannel({ id: '@gotham', name: 'GothamChess' }),
  ],
  'GET /api/playlists': [aPlaylist({ id: 'PLend', name: 'Endgame course' })],
}

describe('ManageSourcesDialog', () => {
  it('shows channels and playlists as chips pressed by membership', async () => {
    renderDialog(chess, library)

    expect(
      within(screen.getByRole('dialog')).getByRole('heading', { level: 2 }).textContent,
    ).toBe('Manage sources in Chess')
    expect({
      channels: await chipsOf('Channels'),
      playlists: await chipsOf('Playlists'),
    }).toEqual({
      channels: [
        ['GothamChess', false],
        ['Hikaru', true],
      ],
      playlists: [['Endgame course', false]],
    })
    expect(
      within(screen.getByRole('button', { name: 'Hikaru' })).getByRole('presentation', {
        hidden: true,
      }),
    ).toHaveAttribute('src', '/avatars/hikaru.jpg')
  })

  it('stages toggles, summarises them and saves the full set at once', async () => {
    const save = vi.fn(() => null)
    const { fetchMock, onOpenChange } = renderDialog(chess, {
      ...library,
      'PUT /api/categories/chess/members': save,
      'GET /api/categories': { categories: [chess], unwatched_total: 0 },
    })
    const user = userEvent.setup()

    await user.click(await screen.findByRole('button', { name: 'GothamChess' }))
    await user.click(screen.getByRole('button', { name: 'Endgame course' }))
    await user.click(screen.getByRole('button', { name: 'Hikaru' }))
    const staged = {
      channels: await chipsOf('Channels'),
      playlists: await chipsOf('Playlists'),
      summary: screen.getByText(/added/).textContent,
      savedYet: save.mock.calls.length,
    }
    await user.click(screen.getByRole('button', { name: 'Save' }))

    expect(staged).toEqual({
      channels: [
        ['GothamChess', true],
        ['Hikaru', false],
      ],
      playlists: [['Endgame course', true]],
      summary: '+2 added, −1 removed',
      savedYet: 0,
    })
    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false))
    const puts = fetchMock.mock.calls.filter(
      ([, init]) => (init as RequestInit | undefined)?.method === 'PUT',
    )
    expect(puts.map(([, init]) => JSON.parse((init as RequestInit).body as string))).toEqual([
      { channels: ['@gotham'], playlists: ['PLend'] },
    ])
  })

  it('filters by search and state, and Enter toggles the only match', async () => {
    renderDialog(aCategory({ id: 'science', name: 'Science', channels: ['@veritasium'] }), {
      'GET /api/channels': [
        aChannel({ id: '@veritasium', name: 'Veritasium' }),
        aChannel({ id: '@hikaru', name: 'Hikaru' }),
        aChannel({ id: '@vsauce', name: 'Vsauce' }),
      ],
      'GET /api/playlists': [aPlaylist({ id: 'PLover', name: 'Overview' })],
    })
    const user = userEvent.setup()
    const search = screen.getByRole('searchbox', { name: 'Search channels and playlists' })
    const focusedOnOpen = search === document.activeElement

    await screen.findByRole('button', { name: 'Vsauce' })
    const filters = screen.getAllByRole('radio').map((filter) => filter.textContent)
    await user.type(search, 'VER')
    const searched = shownChips()
    await user.click(screen.getByRole('radio', { name: /^Not in Science/ }))
    const searchedOut = shownChips()
    await user.click(screen.getByRole('radio', { name: /^In Science/ }))
    const searchedIn = shownChips()
    await user.click(screen.getByRole('radio', { name: 'All' }))
    await user.clear(search)
    await user.type(search, 'hik{Enter}')

    expect(focusedOnOpen).toBe(true)
    expect(filters).toEqual(['All', 'In Science1', 'Not in Science3'])
    expect({ searched, searchedOut, searchedIn }).toEqual({
      searched: ['Veritasium', 'Overview'],
      searchedOut: ['Overview'],
      searchedIn: ['Veritasium'],
    })
    expect(screen.getByRole('button', { name: 'Hikaru' })).toHaveAttribute('aria-pressed', 'true')
    expect(search).toHaveFocus()
    expect(screen.getByRole('radio', { name: /^In Science/ })).toHaveTextContent('In Science2')
  })

  it('disables Save with nothing staged and asks before discarding', async () => {
    const { fetchMock, onOpenChange } = renderDialog(chess, library)
    const user = userEvent.setup()
    const save = screen.getByRole('button', { name: 'Save' })

    const disabledOnOpen = save.hasAttribute('disabled')
    await user.click(await screen.findByRole('button', { name: 'GothamChess' }))
    const disabledWithAChange = save.hasAttribute('disabled')
    await user.click(screen.getByRole('button', { name: 'GothamChess' }))
    const disabledWhenUndone = save.hasAttribute('disabled')
    await user.click(screen.getByRole('button', { name: 'Cancel' }))
    const closedWithoutAsking = onOpenChange.mock.calls.length
    onOpenChange.mockClear()

    await user.click(screen.getByRole('button', { name: 'Hikaru' }))
    await user.keyboard('{Escape}')
    const confirm = await screen.findByRole('dialog', { name: 'Discard changes?' })
    await user.click(within(confirm).getByRole('button', { name: 'Keep editing' }))
    const keptEditing = {
      closed: onOpenChange.mock.calls.length,
      hikaru: screen.getByRole('button', { name: 'Hikaru' }).getAttribute('aria-pressed'),
    }
    await user.click(screen.getByRole('button', { name: 'Close' }))
    await user.click(
      within(await screen.findByRole('dialog', { name: 'Discard changes?' })).getByRole(
        'button',
        { name: 'Discard' },
      ),
    )

    expect([disabledOnOpen, disabledWithAChange, disabledWhenUndone]).toEqual([true, false, true])
    expect(closedWithoutAsking).toBe(1)
    expect(keptEditing).toEqual({ closed: 0, hikaru: 'false' })
    expect(onOpenChange.mock.calls).toEqual([[false]])
    expect(
      fetchMock.mock.calls.filter(([, init]) => (init as RequestInit | undefined)?.method === 'PUT'),
    ).toEqual([])
  })

  it('keeps staged changes when saving fails', async () => {
    const { onOpenChange } = renderDialog(chess, {
      ...library,
      'PUT /api/categories/chess/members': { status: 500, error: 'database is locked' },
    })
    const user = userEvent.setup()

    await user.click(await screen.findByRole('button', { name: 'GothamChess' }))
    await user.click(screen.getByRole('button', { name: 'Save' }))

    expect(await screen.findByText('database is locked')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'GothamChess' })).toHaveAttribute(
      'aria-pressed',
      'true',
    )
    expect(screen.getByText('+1 added')).toBeInTheDocument()
    expect(onOpenChange).not.toHaveBeenCalled()
  })

  it('says when nothing is tracked', async () => {
    renderDialog(chess, {})

    expect(
      await screen.findByText(
        'No channels or playlists to add yet. Track one from the sidebar, then come back.',
      ),
    ).toBeInTheDocument()
    expect(screen.queryAllByRole('toolbar')).toEqual([])
  })
})
