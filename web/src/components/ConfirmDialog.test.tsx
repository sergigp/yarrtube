import { describe, expect, it, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { ConfirmDialog } from './ConfirmDialog'

describe('ConfirmDialog', () => {
  it('renders nothing while closed', () => {
    render(
      <ConfirmDialog
        open={false}
        onOpenChange={() => {}}
        title='Delete "Mix"?'
        onConfirm={() => {}}
      />,
    )

    expect(screen.queryByText('Delete "Mix"?')).not.toBeInTheDocument()
  })

  it('shows the title and description while open', () => {
    render(
      <ConfirmDialog
        open
        onOpenChange={() => {}}
        title='Delete "Mix"?'
        description="This removes the playlist."
        onConfirm={() => {}}
      />,
    )

    expect(screen.getByText('Delete "Mix"?')).toBeInTheDocument()
    expect(screen.getByText('This removes the playlist.')).toBeInTheDocument()
  })

  it('confirms and closes on success', async () => {
    const user = userEvent.setup()
    const onOpenChange = vi.fn()
    const onConfirm = vi.fn().mockResolvedValue(undefined)
    render(
      <ConfirmDialog open onOpenChange={onOpenChange} title='Delete "Mix"?' onConfirm={onConfirm} />,
    )

    await user.click(screen.getByRole('button', { name: 'Delete' }))

    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false))
    expect(onConfirm).toHaveBeenCalledOnce()
  })

  it('shows the failure and stays open so the user can retry', async () => {
    const user = userEvent.setup()
    const onOpenChange = vi.fn()
    const onConfirm = vi.fn().mockRejectedValue(new Error('delete failed'))
    render(
      <ConfirmDialog open onOpenChange={onOpenChange} title='Delete "Mix"?' onConfirm={onConfirm} />,
    )

    await user.click(screen.getByRole('button', { name: 'Delete' }))

    expect(await screen.findByText('delete failed')).toBeInTheDocument()
    expect(onOpenChange).not.toHaveBeenCalledWith(false)
  })

  it('cancels without confirming', async () => {
    const user = userEvent.setup()
    const onOpenChange = vi.fn()
    const onConfirm = vi.fn()
    render(
      <ConfirmDialog open onOpenChange={onOpenChange} title='Delete "Mix"?' onConfirm={onConfirm} />,
    )

    await user.click(screen.getByRole('button', { name: 'Cancel' }))

    expect(onConfirm).not.toHaveBeenCalled()
    expect(onOpenChange).toHaveBeenCalledWith(false)
  })
})
