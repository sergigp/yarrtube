import { useState } from 'react'
import { describe, expect, it, vi } from 'vitest'
import { act, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Route, Routes as RouterRoutes } from 'react-router-dom'
import type { Category } from '@/api/types'
import { queryKeys } from '@/api/queries'
import { CategoryDetail } from './CategoryDetail'
import { CategoryDialog } from './CategoryDialog'
import {
  aCategory,
  aCategoryList,
  aChannel,
  aCreatedCategory,
  aPlaylist,
  aPreferences,
  mockApi,
  renderWithProviders,
  type Routes,
} from '@/test/helpers'
import { RouteProbe } from '@/test/RouteProbe'

function renderDialog(category: Category | undefined, routes: Routes) {
  const fetchMock = mockApi({
    'GET /api/categories': aCategoryList(),
    'GET /api/channels': [],
    'GET /api/playlists': [],
    'GET /api/preferences': aPreferences(),
    ...routes,
  })
  const onOpenChange = vi.fn()
  const { rerender, queryClient } = renderWithProviders(
    <CategoryDialog category={category} open onOpenChange={onOpenChange} />,
  )
  /** Delivers `refetched` to the open dialog, as a library refetch does. */
  const refetch = (refetched: Category) =>
    rerender(<CategoryDialog category={refetched} open onOpenChange={onOpenChange} />)
  return { fetchMock, onOpenChange, refetch, queryClient }
}

/** The dialog to add a category, beside the category view it navigates to. */
function renderCreating(routes: Routes) {
  const fetchMock = mockApi({
    'GET /api/categories': aCategoryList(),
    'GET /api/channels': [],
    'GET /api/playlists': [],
    'GET /api/preferences': aPreferences(),
    ...routes,
  })
  const onOpenChange = vi.fn()
  const Creating = () => {
    const [open, setOpen] = useState(true)
    return (
      <CategoryDialog
        open={open}
        onOpenChange={(next) => {
          setOpen(next)
          onOpenChange(next)
        }}
      />
    )
  }
  renderWithProviders(
    <>
      <Creating />
      <RouterRoutes>
        <Route path="/categories/:id" element={<CategoryDetail />} />
      </RouterRoutes>
      <RouteProbe />
    </>,
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

const categorizationOn = aPreferences({ ai_enabled: true, categorization_enabled: true })

/** The bodies sent with `method`, in order. */
function bodies(fetchMock: ReturnType<typeof mockApi>, method: string): unknown[] {
  return fetchMock.mock.calls
    .filter(([, init]) => (init as RequestInit | undefined)?.method === method)
    .map(([, init]) => JSON.parse((init as RequestInit).body as string))
}

function inDocumentOrder(elements: HTMLElement[]): HTMLElement[] {
  return [...elements].sort((a, b) =>
    a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING ? -1 : 1,
  )
}

const library = {
  'GET /api/channels': [
    aChannel({ id: '@hikaru', name: 'Hikaru', avatar_filename: 'hikaru.jpg' }),
    aChannel({ id: '@gotham', name: 'GothamChess' }),
  ],
  'GET /api/playlists': [aPlaylist({ id: 'PLend', name: 'Endgame course' })],
}

describe('CategoryDialog', () => {
  it('asks for the name, color, prompt and chips when creating, with the next free color', async () => {
    renderDialog(undefined, {
      ...library,
      'GET /api/preferences': categorizationOn,
      'GET /api/categories': aCategoryList([
        aCategory({ name: 'Cooking', color: '#f59e0b' }),
        aCategory({ name: 'Sports', color: '#ef4444' }),
      ]),
    })

    expect(screen.getByRole('dialog', { name: 'Add category' })).toBeVisible()
    await waitFor(() => expect(screen.getByRole('radio', { name: 'Orange' })).toBeChecked())
    const name = screen.getByRole('textbox', { name: 'Name' })
    const color = screen.getByRole('radio', { name: 'Orange' })
    const prompt = await screen.findByRole('textbox', { name: 'AI prompt' })
    const chip = await screen.findByRole('button', { name: 'Hikaru' })
    expect(name).toHaveFocus()
    expect(name).toHaveValue('')
    expect(prompt).toHaveValue('')
    expect(prompt).toHaveAttribute('rows', '6')
    expect(chip).toHaveAttribute('aria-pressed', 'false')
    expect(inDocumentOrder([chip, prompt, color, name])).toEqual([name, color, prompt, chip])
    expect(screen.getByRole('radio', { name: /^In this category/ })).toBeInTheDocument()
    expect(screen.getByRole('radio', { name: /^Not in this category/ })).toBeInTheDocument()
  })

  it('creates everything in one request and opens the category with no dialog open', async () => {
    const { fetchMock } = renderCreating({
      ...library,
      'GET /api/preferences': categorizationOn,
      'POST /api/categories': aCreatedCategory({ id: 'chess', name: 'Chess' }),
      'GET /api/categories': aCategoryList([
        aCategory({ id: 'chess', name: 'Chess', channels: ['@hikaru'] }),
      ]),
      'GET /api/categories/chess/videos': [],
    })
    const user = userEvent.setup()

    await user.type(screen.getByRole('textbox', { name: 'Name' }), '  Chess ')
    await user.click(screen.getByRole('radio', { name: 'Violet' }))
    await user.type(await screen.findByRole('textbox', { name: 'AI prompt' }), ' Chess games ')
    await user.click(await screen.findByRole('button', { name: 'Hikaru' }))
    await user.click(screen.getByRole('button', { name: 'Create' }))

    await waitFor(() =>
      expect(screen.getByRole('status', { name: 'Current route', hidden: true })).toHaveTextContent(
        '/categories/chess',
      ),
    )
    expect(bodies(fetchMock, 'POST')).toEqual([
      {
        name: 'Chess',
        color: '#8b5cf6',
        ai_prompt: 'Chess games',
        channels: ['@hikaru'],
        playlists: [],
      },
    ])
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument())
  })

  it('creates without a prompt field while categorization is off', async () => {
    const { fetchMock } = renderCreating({
      ...library,
      'POST /api/categories': aCreatedCategory({ id: 'chess', name: 'Chess' }),
      'GET /api/categories/chess/videos': [],
    })
    const user = userEvent.setup()

    await user.type(screen.getByRole('textbox', { name: 'Name' }), 'Chess')
    await user.click(await screen.findByRole('button', { name: 'Hikaru' }))
    await user.click(screen.getByRole('button', { name: 'Create' }))

    await waitFor(() => expect(bodies(fetchMock, 'POST')).toHaveLength(1))
    expect(screen.queryByRole('textbox', { name: 'AI prompt' })).not.toBeInTheDocument()
    expect(bodies(fetchMock, 'POST')).toEqual([
      { name: 'Chess', color: '#f59e0b', channels: ['@hikaru'], playlists: [] },
    ])
  })

  it('needs a name to create, and keeps everything when creating fails', async () => {
    const { onOpenChange } = renderCreating({
      ...library,
      'GET /api/preferences': categorizationOn,
      'POST /api/categories': { status: 409, error: 'a category named "Chess" already exists' },
    })
    const user = userEvent.setup()
    const name = screen.getByRole('textbox', { name: 'Name' })
    const create = screen.getByRole('button', { name: 'Create' })

    expect(create).toBeDisabled()
    await user.type(name, '   ')
    expect(create).toBeDisabled()

    await user.type(name, 'Chess')
    await user.type(await screen.findByRole('textbox', { name: 'AI prompt' }), 'Chess games')
    await user.click(await screen.findByRole('button', { name: 'Hikaru' }))
    await user.click(create)

    expect(await screen.findByText('a category named "Chess" already exists')).toBeInTheDocument()
    expect(onOpenChange).not.toHaveBeenCalled()
    expect(name).toHaveValue('   Chess')
    expect(screen.getByRole('textbox', { name: 'AI prompt' })).toHaveValue('Chess games')
    expect(screen.getByRole('button', { name: 'Hikaru' })).toHaveAttribute('aria-pressed', 'true')
  })

  it('neither creates nor closes on Enter in the color picker', async () => {
    const { fetchMock, onOpenChange } = renderCreating(library)
    const user = userEvent.setup()

    await user.type(screen.getByRole('textbox', { name: 'Name' }), 'Chess')
    await user.click(screen.getByRole('radio', { name: 'Violet' }))
    await user.keyboard('{Enter}')
    await user.click(screen.getByRole('radio', { name: 'Custom' }))
    await user.click(screen.getByRole('slider', { name: 'Hue' }))
    await user.keyboard('{Enter}')

    expect(onOpenChange).not.toHaveBeenCalled()
    expect(bodies(fetchMock, 'POST')).toEqual([])
  })

  it('closes an untouched new category without asking', async () => {
    const { onOpenChange } = renderCreating({})
    const user = userEvent.setup()

    await user.click(screen.getByRole('button', { name: 'Cancel' }))

    expect(onOpenChange).toHaveBeenCalledWith(false)
    expect(screen.queryByText("This category won't be created.")).not.toBeInTheDocument()
  })

  it('asks before discarding a new category', async () => {
    const { fetchMock, onOpenChange } = renderCreating({})
    const user = userEvent.setup()

    await user.type(screen.getByRole('textbox', { name: 'Name' }), 'Chess')
    await user.click(screen.getByRole('button', { name: 'Cancel' }))

    expect(await screen.findByText("This category won't be created.")).toBeInTheDocument()
    expect(onOpenChange).not.toHaveBeenCalled()
    await user.click(screen.getByRole('button', { name: 'Discard' }))
    expect(onOpenChange).toHaveBeenCalledWith(false)
    expect(bodies(fetchMock, 'POST')).toEqual([])
  })

  it('prefills the name and color, and saves them in one update without the members', async () => {
    const violetChess = { ...chess, color: '#8b5cf6' }
    const { fetchMock, onOpenChange } = renderDialog(violetChess, {
      'PATCH /api/categories/chess': () => null,
    })
    const user = userEvent.setup()
    const name = screen.getByRole('textbox', { name: 'Name' })

    expect(name).toHaveValue('Chess')
    expect(name).toHaveFocus()
    expect(screen.getByRole('radio', { name: 'Violet' })).toBeChecked()
    await user.clear(name)
    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled()
    await user.type(name, 'Board games')
    await user.click(screen.getByRole('radio', { name: 'Teal' }))
    await user.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false))
    expect(bodies(fetchMock, 'PATCH')).toEqual([{ name: 'Board games', color: '#14b8a6' }])
    expect(bodies(fetchMock, 'PUT')).toEqual([])
  })

  it('stays open without sending the members when the name is taken', async () => {
    const { fetchMock, onOpenChange } = renderDialog(chess, {
      ...library,
      'PATCH /api/categories/chess': {
        status: 409,
        error: 'a category named "Cooking" already exists',
      },
    })
    const user = userEvent.setup()
    const name = screen.getByRole('textbox', { name: 'Name' })

    await user.clear(name)
    await user.type(name, 'Cooking')
    await user.click(await screen.findByRole('button', { name: 'GothamChess' }))
    await user.click(screen.getByRole('button', { name: 'Save' }))

    expect(await screen.findByText('a category named "Cooking" already exists')).toBeInTheDocument()
    expect(onOpenChange).not.toHaveBeenCalled()
    expect(name).toHaveValue('Cooking')
    expect(bodies(fetchMock, 'PUT')).toEqual([])
  })

  it('saves on Enter in the name, but neither toggles nor saves on Enter with several matches', async () => {
    const { fetchMock, onOpenChange } = renderDialog(chess, {
      ...library,
      'PATCH /api/categories/chess': () => null,
    })
    const user = userEvent.setup()
    const name = screen.getByRole('textbox', { name: 'Name' })
    const search = screen.getByRole('searchbox', { name: 'Search channels and playlists' })

    await screen.findByRole('button', { name: 'GothamChess' })
    await user.type(name, ' club')
    await user.type(search, 'h{Enter}')

    expect(onOpenChange).not.toHaveBeenCalled()
    expect(bodies(fetchMock, 'PATCH')).toEqual([])
    expect(await chipsOf('Channels')).toEqual([
      ['GothamChess', false],
      ['Hikaru', true],
    ])

    await user.type(name, '{Enter}')

    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false))
    expect(bodies(fetchMock, 'PATCH')).toEqual([{ name: 'Chess club' }])
  })

  it('keeps the dialog and what was staged when closing the prompt guide', async () => {
    const { onOpenChange } = renderDialog(chess, {
      ...library,
      'GET /api/preferences': categorizationOn,
    })
    const user = userEvent.setup()

    await user.click(await screen.findByRole('button', { name: 'GothamChess' }))
    await user.click(screen.getByRole('button', { name: 'About AI prompt' }))
    const guide = await screen.findByRole('region', { name: 'AI prompt guide' })
    await user.keyboard('{Escape}')

    await waitFor(() => expect(guide).not.toBeInTheDocument())
    expect(onOpenChange).not.toHaveBeenCalled()
    expect(screen.queryByRole('dialog', { name: 'Discard changes?' })).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'GothamChess' })).toHaveAttribute(
      'aria-pressed',
      'true',
    )
  })

  it('shows channels and playlists as chips pressed by membership', async () => {
    renderDialog(chess, library)

    expect(
      within(screen.getByRole('dialog')).getByRole('heading', { level: 2 }).textContent,
    ).toBe('Edit Chess settings')
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

    await screen.findByRole('button', { name: 'Vsauce' })
    const filters = within(screen.getByLabelText('Show'))
      .getAllByRole('radio')
      .map((filter) => filter.textContent)
    await user.type(search, 'VER')
    const searched = shownChips()
    await user.click(screen.getByRole('radio', { name: /^Not in Science/ }))
    const searchedOut = shownChips()
    await user.click(screen.getByRole('radio', { name: /^In Science/ }))
    const searchedIn = shownChips()
    await user.click(screen.getByRole('radio', { name: 'All' }))
    await user.clear(search)
    await user.type(search, 'hik{Enter}')

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

  it('shows the AI prompt prefilled while AI categorization is available', async () => {
    renderDialog(aCategory({ ...chess, ai_prompt: 'Chess games' }), {
      ...library,
      'GET /api/preferences': categorizationOn,
    })

    const prompt = await screen.findByRole('textbox', { name: 'AI prompt' })

    expect(prompt).toHaveValue('Chess games')
    expect(prompt).toHaveAccessibleDescription('Describe the videos that belong in this category.')
  })

  it('hides the AI prompt otherwise, leaving it out of a save', async () => {
    const { fetchMock, onOpenChange } = renderDialog(
      aCategory({ ...chess, ai_prompt: 'Chess games' }),
      { ...library, 'PUT /api/categories/chess/members': () => null },
    )
    const user = userEvent.setup()

    await user.click(await screen.findByRole('button', { name: 'GothamChess' }))
    await user.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false))
    expect(screen.queryByRole('textbox', { name: 'AI prompt' })).not.toBeInTheDocument()
    expect(bodies(fetchMock, 'PATCH')).toEqual([])
  })

  it('saves only the AI prompt when the sources are untouched', async () => {
    const { fetchMock, onOpenChange } = renderDialog(chess, {
      ...library,
      'GET /api/preferences': categorizationOn,
      'PATCH /api/categories/chess': () => null,
    })
    const user = userEvent.setup()

    await user.type(await screen.findByRole('textbox', { name: 'AI prompt' }), ' Chess games ')
    await user.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false))
    expect(bodies(fetchMock, 'PATCH')).toEqual([{ ai_prompt: 'Chess games' }])
    expect(bodies(fetchMock, 'PUT')).toEqual([])
  })

  it('saves both the AI prompt and the sources', async () => {
    const { fetchMock, onOpenChange } = renderDialog(chess, {
      ...library,
      'GET /api/preferences': categorizationOn,
      'PATCH /api/categories/chess': () => null,
      'PUT /api/categories/chess/members': () => null,
    })
    const user = userEvent.setup()

    await user.type(await screen.findByRole('textbox', { name: 'AI prompt' }), 'Chess games')
    await user.click(await screen.findByRole('button', { name: 'GothamChess' }))
    await user.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false))
    expect(bodies(fetchMock, 'PATCH')).toEqual([{ ai_prompt: 'Chess games' }])
    expect(bodies(fetchMock, 'PUT')).toEqual([{ channels: ['@hikaru', '@gotham'], playlists: [] }])
  })

  it('removes the AI prompt when emptied', async () => {
    const { fetchMock, onOpenChange } = renderDialog(
      aCategory({ ...chess, ai_prompt: 'Chess games' }),
      {
        ...library,
        'GET /api/preferences': categorizationOn,
        'PATCH /api/categories/chess': () => null,
      },
    )
    const user = userEvent.setup()

    await user.clear(await screen.findByRole('textbox', { name: 'AI prompt' }))
    await user.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false))
    expect(bodies(fetchMock, 'PATCH')).toEqual([{ ai_prompt: '' }])
  })

  it('counts an edited AI prompt as a change to save or discard', async () => {
    const { fetchMock, onOpenChange } = renderDialog(chess, {
      ...library,
      'GET /api/preferences': categorizationOn,
    })
    const user = userEvent.setup()
    const save = screen.getByRole('button', { name: 'Save' })

    await user.type(await screen.findByRole('textbox', { name: 'AI prompt' }), 'Chess')
    expect(save).toBeEnabled()
    await user.click(screen.getByRole('button', { name: 'Cancel' }))

    const confirm = await screen.findByRole('dialog', { name: 'Discard changes?' })
    expect(onOpenChange).not.toHaveBeenCalled()
    await user.click(within(confirm).getByRole('button', { name: 'Discard' }))
    expect(onOpenChange.mock.calls).toEqual([[false]])
    expect(bodies(fetchMock, 'PATCH')).toEqual([])
  })

  it('keeps the AI prompt when saving it fails', async () => {
    const { onOpenChange } = renderDialog(chess, {
      ...library,
      'GET /api/preferences': categorizationOn,
      'PATCH /api/categories/chess': { status: 400, error: 'too long' },
    })
    const user = userEvent.setup()

    await user.type(await screen.findByRole('textbox', { name: 'AI prompt' }), 'Chess games')
    await user.click(screen.getByRole('button', { name: 'Save' }))

    expect(await screen.findByText('too long')).toBeInTheDocument()
    expect(screen.getByRole('textbox', { name: 'AI prompt' })).toHaveValue('Chess games')
    expect(onOpenChange).not.toHaveBeenCalled()
  })

  it('leaves a prompt saved elsewhere alone when saving the sources', async () => {
    const prompted = aCategory({ ...chess, ai_prompt: 'Chess games' })
    const { fetchMock, onOpenChange, refetch } = renderDialog(prompted, {
      ...library,
      'GET /api/preferences': categorizationOn,
      'PUT /api/categories/chess/members': () => null,
    })
    const user = userEvent.setup()
    await screen.findByRole('textbox', { name: 'AI prompt' })

    refetch({ ...prompted, ai_prompt: 'Openings only' })
    await user.click(await screen.findByRole('button', { name: 'GothamChess' }))
    await user.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false))
    expect(bodies(fetchMock, 'PATCH')).toEqual([])
  })

  it('stages nothing when a hidden prompt changes elsewhere', async () => {
    const prompted = aCategory({ ...chess, ai_prompt: 'Chess games' })
    const { refetch } = renderDialog(prompted, library)
    await screen.findByRole('button', { name: 'GothamChess' })

    refetch({ ...prompted, ai_prompt: 'Openings only' })

    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled()
  })

  it('leaves out an edited prompt once its field is hidden', async () => {
    const { fetchMock, onOpenChange, queryClient } = renderDialog(chess, {
      ...library,
      'GET /api/preferences': categorizationOn,
      'PUT /api/categories/chess/members': () => null,
    })
    const user = userEvent.setup()

    await user.type(await screen.findByRole('textbox', { name: 'AI prompt' }), 'Chess games')
    act(() => queryClient.setQueryData(queryKeys.preferences, aPreferences()))
    await waitFor(() =>
      expect(screen.queryByRole('textbox', { name: 'AI prompt' })).not.toBeInTheDocument(),
    )
    await user.click(await screen.findByRole('button', { name: 'GothamChess' }))
    await user.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false))
    expect(bodies(fetchMock, 'PATCH')).toEqual([])
  })

  it('sends a pasted prompt whole, for the daemon to judge its length', async () => {
    const { fetchMock } = renderDialog(chess, {
      ...library,
      'GET /api/preferences': categorizationOn,
      'PATCH /api/categories/chess': { status: 400, error: 'too long' },
    })
    const user = userEvent.setup()
    const pasted = 'x'.repeat(2500)

    await user.click(await screen.findByRole('textbox', { name: 'AI prompt' }))
    await user.paste(pasted)
    await user.click(screen.getByRole('button', { name: 'Save' }))

    expect(await screen.findByText('too long')).toBeInTheDocument()
    expect(bodies(fetchMock, 'PATCH')).toEqual([{ ai_prompt: pasted }])
  })

  it('says the other changes were saved when only the sources failed, and retries the sources', async () => {
    let failures = 1
    const { fetchMock, onOpenChange } = renderDialog(chess, {
      ...library,
      'GET /api/preferences': categorizationOn,
      'PATCH /api/categories/chess': () => null,
      'PUT /api/categories/chess/members': () =>
        failures-- > 0 ? { status: 500, error: 'database is locked' } : null,
    })
    const user = userEvent.setup()

    await user.type(await screen.findByRole('textbox', { name: 'AI prompt' }), 'Chess games')
    await user.click(await screen.findByRole('button', { name: 'GothamChess' }))
    await user.click(screen.getByRole('button', { name: 'Save' }))

    expect(
      await screen.findByText(
        "Your other changes were saved, but the sources weren't: database is locked",
      ),
    ).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false))
    expect(bodies(fetchMock, 'PATCH')).toEqual([{ ai_prompt: 'Chess games' }])
    expect(bodies(fetchMock, 'PUT')).toHaveLength(2)
  })

  it('says the other changes stay saved when discarding after only the sources failed', async () => {
    const { onOpenChange } = renderDialog(chess, {
      ...library,
      'GET /api/preferences': categorizationOn,
      'PATCH /api/categories/chess': () => null,
      'PUT /api/categories/chess/members': { status: 500, error: 'database is locked' },
    })
    const user = userEvent.setup()

    await user.type(await screen.findByRole('textbox', { name: 'AI prompt' }), 'Chess games')
    await user.click(await screen.findByRole('button', { name: 'GothamChess' }))
    await user.click(screen.getByRole('button', { name: 'Save' }))
    await screen.findByText(/database is locked/)
    await user.click(screen.getByRole('button', { name: 'Cancel' }))

    const confirm = await screen.findByRole('dialog', { name: 'Discard changes?' })
    expect(confirm).toHaveTextContent(
      "Your other changes are already saved. Your changes to the sources in Chess won't be saved.",
    )
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

  it('re-evaluates N videos after confirmation, hidden at 0', async () => {
    const reevaluate = vi.fn(() => null)
    const evaluated = { ...chess, ai_prompt: 'Chess videos', ai_verdict_count: 40 }
    const { refetch } = renderDialog(evaluated, {
      ...library,
      'GET /api/preferences': categorizationOn,
      'DELETE /api/categories/chess/ai-verdicts': reevaluate,
      'GET /api/categories': { categories: [evaluated], unwatched_total: 0 },
    })
    const user = userEvent.setup()

    await user.click(await screen.findByRole('button', { name: 'Re-evaluate 40 videos' }))
    const confirmation = await screen.findByRole('dialog', { name: 'Re-evaluate 40 videos?' })
    expect(
      within(confirmation).getByText(
        'The AI will evaluate these 40 videos for Chess again. Categories you set by hand stay.',
      ),
    ).toBeInTheDocument()
    await user.click(within(confirmation).getByRole('button', { name: 'Re-evaluate' }))

    await waitFor(() => expect(reevaluate).toHaveBeenCalledOnce())
    refetch({ ...evaluated, ai_verdict_count: 0 })
    expect(screen.queryByRole('button', { name: /^Re-evaluate/ })).not.toBeInTheDocument()
  })

  it('keeps re-evaluate disabled while the prompt has unsaved edits', async () => {
    renderDialog(
      { ...chess, ai_prompt: 'Chess videos', ai_verdict_count: 40 },
      { ...library, 'GET /api/preferences': categorizationOn },
    )
    const user = userEvent.setup()

    await user.type(await screen.findByRole('textbox', { name: 'AI prompt' }), ' only')

    expect(screen.getByRole('button', { name: 'Re-evaluate 40 videos' })).toBeDisabled()
    expect(screen.getByText('Save the prompt first to re-evaluate with it.')).toBeInTheDocument()
  })

  it('names a single video to re-evaluate in the singular', async () => {
    renderDialog(
      { ...chess, ai_prompt: 'Chess videos', ai_verdict_count: 1 },
      { ...library, 'GET /api/preferences': categorizationOn },
    )
    const user = userEvent.setup()

    await user.click(await screen.findByRole('button', { name: 'Re-evaluate 1 video' }))

    expect(
      await screen.findByText(
        'The AI will evaluate this video for Chess again. Categories you set by hand stay.',
      ),
    ).toBeInTheDocument()
  })
})
