import { describe, expect, it, vi } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { EditChannelDialog } from './EditChannelDialog'
import {
  aCategory,
  aCategoryList,
  aChannel,
  mockApi,
  renderWithProviders,
  type Routes,
} from '@/test/helpers'
import type { ChannelListItem } from '@/api/types'

function renderDialog(
  channel: ChannelListItem,
  onSave: (changes: object) => Promise<void> = vi.fn(async () => {}),
  routes: Routes = { 'GET /api/categories': aCategoryList() },
) {
  const fetchMock = mockApi(routes)
  const onOpenChange = vi.fn()
  renderWithProviders(
    <EditChannelDialog channel={channel} open onOpenChange={onOpenChange} onSave={onSave} />,
  )
  return { onOpenChange, onSave, fetchMock }
}

describe('EditChannelDialog', () => {
  it("opens prefilled with the channel's current settings", () => {
    renderDialog(aChannel({ name: 'Veritasium', quality: 'mid', video_limit: 5 }))

    expect(screen.getByRole('dialog', { name: 'Edit Veritasium settings' })).toBeInTheDocument()
    expect(screen.getByRole('combobox', { name: /Video quality/ })).toHaveTextContent('Mid')
    expect(screen.getByLabelText('Video limit')).toHaveValue(5)
  })

  it("opens with the channel's interval in hours", () => {
    renderDialog(aChannel({ reconcile_interval_minutes: 180 }))

    expect(screen.getByLabelText('Check for new videos every')).toHaveValue(3)
    expect(screen.getByRole('combobox', { name: 'Interval unit' })).toHaveTextContent('hours')
  })

  it('sends only the changed interval, in minutes', async () => {
    const { onSave, onOpenChange } = renderDialog(aChannel({ reconcile_interval_minutes: 180 }))
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
  it('sends only the changed quality', async () => {
    const { onSave, onOpenChange } = renderDialog(aChannel({ quality: 'high', video_limit: 5 }))
    const user = userEvent.setup()

    await user.click(screen.getByRole('combobox', { name: /Video quality/ }))
    await user.click(await screen.findByRole('option', { name: 'Low' }))
    await user.click(screen.getByRole('button', { name: 'Save' }))

    expect(onSave).toHaveBeenCalledExactlyOnceWith({ quality: 'low' })
    expect(onOpenChange).toHaveBeenCalledWith(false)
  })

  it('notes that a changed quality applies to new videos only', async () => {
    renderDialog(aChannel({ quality: 'high' }))
    const user = userEvent.setup()

    await user.click(screen.getByRole('combobox', { name: /Video quality/ }))
    await user.click(await screen.findByRole('option', { name: 'Low' }))

    expect(
      screen.getByText(
        'The new quality applies to new videos only. Videos already downloaded keep their current quality.',
      ),
    ).toBeInTheDocument()
  })

  it('does not note anything while the quality is unchanged', () => {
    renderDialog(aChannel({ quality: 'high' }))

    expect(screen.queryByText(/The new quality applies/)).not.toBeInTheDocument()
  })

  it('sends nothing and closes when nothing changed', async () => {
    const { onSave, onOpenChange } = renderDialog(aChannel({ quality: 'high', video_limit: 5 }))
    const user = userEvent.setup()

    await user.click(screen.getByRole('button', { name: 'Save' }))

    expect(onSave).not.toHaveBeenCalled()
    expect(onOpenChange).toHaveBeenCalledWith(false)
  })

  it('warns when lowering the video limit', async () => {
    renderDialog(aChannel({ video_limit: 10 }))
    const user = userEvent.setup()
    const limit = screen.getByLabelText('Video limit')

    await user.clear(limit)
    await user.type(limit, '3')

    expect(
      screen.getByText(
        'The next sync deletes the downloaded videos of this channel beyond the 3 newest.',
      ),
    ).toBeInTheDocument()
  })

  it('does not warn when raising the video limit', async () => {
    renderDialog(aChannel({ video_limit: 10 }))
    const user = userEvent.setup()
    const limit = screen.getByLabelText('Video limit')

    await user.clear(limit)
    await user.type(limit, '20')

    expect(screen.queryByText(/The next sync deletes/)).not.toBeInTheDocument()
  })

  it('does not submit an out-of-range video limit', async () => {
    const { onSave, onOpenChange } = renderDialog(aChannel({ video_limit: 10 }))
    const user = userEvent.setup()
    const limit = screen.getByLabelText('Video limit')

    await user.clear(limit)
    await user.type(limit, '1001')
    await user.click(screen.getByRole('button', { name: 'Save' }))

    expect(limit).toBeInvalid()
    expect(onSave).not.toHaveBeenCalled()
    expect(onOpenChange).not.toHaveBeenCalled()
  })

  it('does not submit an invalid interval', async () => {
    const { onSave, onOpenChange } = renderDialog(aChannel({ reconcile_interval_minutes: 180 }))
    const user = userEvent.setup()
    const amount = screen.getByLabelText('Check for new videos every')

    await user.clear(amount)
    await user.type(amount, '0')
    await user.click(screen.getByRole('button', { name: 'Save' }))

    expect(amount).toBeInvalid()
    expect(onSave).not.toHaveBeenCalled()
    expect(onOpenChange).not.toHaveBeenCalled()
  })

  it('keeps the dialog open and shows the error when saving fails', async () => {
    const onSave = vi.fn(async () => {
      throw new Error('channel @x not found')
    })
    const { onOpenChange } = renderDialog(aChannel({ video_limit: 10 }), onSave)
    const user = userEvent.setup()
    const limit = screen.getByLabelText('Video limit')

    await user.clear(limit)
    await user.type(limit, '20')
    await user.click(screen.getByRole('button', { name: 'Save' }))

    expect(await screen.findByText('channel @x not found')).toBeInTheDocument()
    expect(onOpenChange).not.toHaveBeenCalled()
  })

  it('sends nothing when the dialog is closed without saving', async () => {
    const { onSave, onOpenChange } = renderDialog(aChannel({ video_limit: 10 }))
    const user = userEvent.setup()
    const limit = screen.getByLabelText('Video limit')

    await user.clear(limit)
    await user.type(limit, '20')
    await user.click(screen.getByRole('button', { name: 'Close' }))

    expect(onOpenChange).toHaveBeenCalledWith(false)
    expect(onSave).not.toHaveBeenCalled()
  })

  it('saves a change of categories alone', async () => {
    const { onSave, onOpenChange, fetchMock } = renderDialog(
      aChannel({ id: '@hikaru', name: 'Hikaru' }),
      vi.fn(async () => {}),
      {
        'GET /api/categories': aCategoryList([
          aCategory({ id: 'chess', name: 'Chess', channels: ['@gotham', '@hikaru'] }),
          aCategory({ id: 'cooking', name: 'Cooking' }),
          aCategory({ id: 'streams', name: 'Streams', channels: ['@hikaru'] }),
        ]),
        'PUT /api/categories/chess/members': null,
      },
    )
    const user = userEvent.setup()

    await user.click(await screen.findByRole('button', { name: 'Remove Chess' }))
    await user.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false))
    expect(onSave).not.toHaveBeenCalled()
    expect(
      fetchMock.mock.calls
        .filter(([, init]) => (init as RequestInit | undefined)?.method === 'PUT')
        .map(([url, init]) => [url, JSON.parse((init as RequestInit).body as string)]),
    ).toEqual([['/api/categories/chess/members', { channels: ['@gotham'], playlists: [] }]])
  })

  it('picks a category with Enter without saving the dialog', async () => {
    const { onSave, onOpenChange } = renderDialog(
      aChannel({ id: '@hikaru', name: 'Hikaru' }),
      vi.fn(async () => {}),
      { 'GET /api/categories': aCategoryList([aCategory({ id: 'chess', name: 'Chess' })]) },
    )
    const field = screen.getByRole('combobox', { name: 'Categories' })
    await screen.findByText('Categories')

    await userEvent.setup().type(field, 'che{Enter}')

    expect(
      await within(screen.getByRole('list', { name: 'Chosen categories' })).findByText('Chess'),
    ).toBeInTheDocument()
    expect(onOpenChange).not.toHaveBeenCalled()
    expect(onSave).not.toHaveBeenCalled()
  })
  it('shows where the channel is stored', () => {
    renderDialog(aChannel({ name: 'Veritasium', path: 'channels/veritasium' }))

    expect(screen.getByText('Location')).toBeInTheDocument()
    expect(screen.getByText('channels/veritasium')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Move…' })).toBeInTheDocument()
  })

  it('opens the move dialog without saving', async () => {
    const { onSave } = renderDialog(aChannel({ name: 'Veritasium', path: 'channels/veritasium' }), vi.fn(async () => {}), {
      'GET /api/categories': aCategoryList(),
      'GET /api/playlists': [],
      'GET /api/channels': [],
      'GET /api/directories': { root: '/videos', path: '', entries: [] },
    })
    const user = userEvent.setup()

    await user.click(screen.getByRole('button', { name: 'Move…' }))

    expect(await screen.findByRole('dialog', { name: 'Move Veritasium' })).toBeInTheDocument()
    expect(screen.queryByRole('dialog', { name: 'Edit Veritasium settings' })).not.toBeInTheDocument()
    expect(await screen.findByRole('radio', { name: 'channels/' })).toBeChecked()
    expect(onSave).not.toHaveBeenCalled()
  })

  it('focuses its first setting, not "Move…", when opened', () => {
    renderDialog(aChannel())

    expect(screen.getByRole('combobox', { name: /Video quality/ })).toHaveFocus()
  })

  it('disables "Move…" while the channel has work in progress', () => {
    renderDialog(aChannel({ work_in_progress: true }))

    expect(screen.getByRole('button', { name: 'Move…' })).toBeDisabled()
    expect(
      screen.getByText('Downloads in progress — you can move it once they finish.'),
    ).toBeInTheDocument()
  })
})
