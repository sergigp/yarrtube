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
})
