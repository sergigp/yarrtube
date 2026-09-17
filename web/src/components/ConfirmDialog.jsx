import { useState } from 'react'
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'

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
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="sm:max-w-sm">
        <DialogTitle>{title}</DialogTitle>
        {description && <DialogDescription>{description}</DialogDescription>}
        {error && <p className="text-sm text-destructive">{error.message}</p>}
        <div className="flex justify-end gap-2">
          <Button type="button" variant="outline" disabled={confirming} onClick={() => handleOpenChange(false)}>
            Cancel
          </Button>
          <Button type="button" variant="destructive" onClick={handleConfirm} disabled={confirming}>
            {confirming ? 'Deleting…' : confirmLabel}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}
