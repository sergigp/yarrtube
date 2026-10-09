import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { EditChannelDialog } from './EditChannelDialog'
import { aChannel } from '@/test/helpers'
import type { ChannelListItem } from '@/api/types'

function renderDialog(
  channel: ChannelListItem,
  onSave: (changes: object) => Promise<void> = vi.fn(async () => {}),
) {
  const onOpenChange = vi.fn()
  render(<EditChannelDialog channel={channel} open onOpenChange={onOpenChange} onSave={onSave} />)
  return { onOpenChange, onSave }
}

describe('EditChannelDialog', () => {
  it("opens prefilled with the channel's current settings", () => {
    renderDialog(aChannel({ name: 'Veritasium', quality: 'mid', video_limit: 5 }))

    expect(screen.getByRole('dialog', { name: 'Edit Veritasium settings' })).toBeInTheDocument()
    expect(screen.getByRole('combobox', { name: /Video quality/ })).toHaveTextContent('Mid')
    expect(screen.getByLabelText('Video limit')).toHaveValue(5)
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
})
