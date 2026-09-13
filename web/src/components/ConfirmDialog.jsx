import { useState } from 'react'
import * as Dialog from '@radix-ui/react-dialog'

/**
 * Shared destructive-action confirmation dialog. `onConfirm` may return a
 * promise; while it's pending the confirm button is disabled and the dialog
 * stays open, closing automatically only on success. A failure is shown
 * inline and the dialog stays open so the user can retry or cancel.
 */
export function ConfirmDialog({
  open,
  onOpenChange,
  title,
  description,
  confirmLabel = 'Delete',
  onConfirm,
}) {
  const [error, setError] = useState(null)
  const [confirming, setConfirming] = useState(false)

  const handleOpenChange = (nextOpen) => {
    if (!confirming) {
      setError(null)
      onOpenChange(nextOpen)
    }
  }

  const handleConfirm = async () => {
    setConfirming(true)
    setError(null)
    try {
      await onConfirm()
      onOpenChange(false)
    } catch (err) {
      setError(err)
    } finally {
      setConfirming(false)
    }
  }

  return (
    <Dialog.Root open={open} onOpenChange={handleOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="dialog-overlay" />
        <Dialog.Content className="dialog-content confirm-dialog">
          <Dialog.Title className="dialog-title">{title}</Dialog.Title>
          {description && (
            <Dialog.Description className="confirm-dialog-description">
              {description}
            </Dialog.Description>
          )}
          {error && <p className="error">{error.message}</p>}
          <div className="confirm-dialog-actions">
            <Dialog.Close asChild>
              <button type="button" className="secondary-button" disabled={confirming}>
                Cancel
              </button>
            </Dialog.Close>
            <button
              type="button"
              className="danger-button"
              onClick={handleConfirm}
              disabled={confirming}
            >
              {confirming ? 'Deleting…' : confirmLabel}
            </button>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  )
}
