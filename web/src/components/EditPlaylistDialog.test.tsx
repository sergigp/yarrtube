import { describe, expect, it, vi } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { EditPlaylistDialog } from './EditPlaylistDialog'
import {
  aCategory,
  aCategoryList,
  aPlaylist,
  mockApi,
  renderWithProviders,
  type Routes,
} from '@/test/helpers'
import type { PlaylistListItem } from '@/api/types'

function renderDialog(
  playlist: PlaylistListItem,
  onSave: (changes: object) => Promise<void> = vi.fn(async () => {}),
  routes: Routes = { 'GET /api/categories': aCategoryList() },
) {
  const fetchMock = mockApi(routes)
  const onOpenChange = vi.fn()
  renderWithProviders(
    <EditPlaylistDialog playlist={playlist} open onOpenChange={onOpenChange} onSave={onSave} />,
  )
  return { onOpenChange, onSave, fetchMock }
}

describe('EditPlaylistDialog', () => {
  it("opens prefilled with the playlist's interval", () => {
    renderDialog(aPlaylist({ name: 'Lofi beats', reconcile_interval_minutes: 180 }))

    expect(screen.getByRole('dialog', { name: 'Edit Lofi beats settings' })).toBeInTheDocument()
    expect(screen.getByLabelText('Check for new videos every')).toHaveValue(3)
    expect(screen.getByRole('combobox', { name: 'Interval unit' })).toHaveTextContent('hours')
  })

  it('sends the changed interval, in minutes, and closes', async () => {
    const { onSave, onOpenChange } = renderDialog(aPlaylist({ reconcile_interval_minutes: 180 }))
    const user = userEvent.setup()
    const amount = screen.getByLabelText('Check for new videos every')

    await user.clear(amount)
    await user.type(amount, '30')
    await user.click(screen.getByRole('combobox', { name: 'Interval unit' }))
    await user.click(await screen.findByRole('option', { name: 'minutes' }))
    await user.click(screen.getByRole('button', { name: 'Save' }))

    expect(onSave).toHaveBeenCalledExactlyOnceWith({ reconcile_interval_minutes: 30 })
    expect(onOpenChange).toHaveBeenCalledWith(false)
  })

  it('sends nothing and closes when nothing changed', async () => {
    const { onSave, onOpenChange } = renderDialog(aPlaylist({ reconcile_interval_minutes: 180 }))
    const user = userEvent.setup()

    await user.click(screen.getByRole('button', { name: 'Save' }))

    expect(onSave).not.toHaveBeenCalled()
    expect(onOpenChange).toHaveBeenCalledWith(false)
  })

  it('keeps the dialog open and shows the error when saving fails', async () => {
    const onSave = vi.fn(async () => {
      throw new Error('playlist PL1 not found')
    })
    const { onOpenChange } = renderDialog(aPlaylist({ reconcile_interval_minutes: 180 }), onSave)
    const user = userEvent.setup()
    const amount = screen.getByLabelText('Check for new videos every')

    await user.clear(amount)
    await user.type(amount, '6')
    await user.click(screen.getByRole('button', { name: 'Save' }))

    expect(await screen.findByText('playlist PL1 not found')).toBeInTheDocument()
    expect(onOpenChange).not.toHaveBeenCalled()
  })

  it('does not submit more than 24 hours', async () => {
    const { onSave, onOpenChange } = renderDialog(aPlaylist({ reconcile_interval_minutes: 180 }))
    const user = userEvent.setup()
    const amount = screen.getByLabelText('Check for new videos every')

    await user.clear(amount)
    await user.type(amount, '25')
    await user.click(screen.getByRole('button', { name: 'Save' }))

    expect(amount).toBeInvalid()
    expect(onSave).not.toHaveBeenCalled()
    expect(onOpenChange).not.toHaveBeenCalled()
  })

  it('saves a change of categories alone', async () => {
    const { onSave, onOpenChange, fetchMock } = renderDialog(
      aPlaylist({ id: 'PLend', name: 'Endgames' }),
      vi.fn(async () => {}),
      {
        'GET /api/categories': aCategoryList([
          aCategory({ id: 'chess', name: 'Chess', playlists: ['PLend'] }),
          aCategory({ id: 'courses', name: 'Courses', channels: ['@teacher'] }),
        ]),
        'PUT /api/categories/courses/members': null,
      },
    )
    const user = userEvent.setup()
    await screen.findByText('Chess')

    await user.type(screen.getByRole('combobox', { name: 'Categories' }), 'cour')
    await user.click(await screen.findByRole('option', { name: 'Courses' }))
    await user.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false))
    expect(onSave).not.toHaveBeenCalled()
    expect(
      fetchMock.mock.calls
        .filter(([, init]) => (init as RequestInit | undefined)?.method === 'PUT')
        .map(([url, init]) => [url, JSON.parse((init as RequestInit).body as string)]),
    ).toEqual([
      ['/api/categories/courses/members', { channels: ['@teacher'], playlists: ['PLend'] }],
    ])
  })
})
