import { describe, expect, it, vi } from 'vitest'
import { screen, within } from '@testing-library/react'
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
})
