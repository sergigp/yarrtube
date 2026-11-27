import { ConfirmDialog } from './ConfirmDialog'

interface UnfavoriteKeptDialogProps {
  open: boolean
  /** Names the video. */
  title: string
  /** Names the channel or playlist that keeps it. */
  sourceName: string
  onConfirm: () => void | Promise<void>
  onCancel: () => void
}

/**
 * Asks before unfavoriting a kept video: no longer in its channel or
 * playlist, it leaves at the next sync, its files deleted.
 */
export function UnfavoriteKeptDialog({
  open,
  title,
  sourceName,
  onConfirm,
  onCancel,
}: UnfavoriteKeptDialogProps) {
  return (
    <ConfirmDialog
      open={open}
      onOpenChange={(nextOpen) => {
        if (!nextOpen) onCancel()
      }}
      title={`Remove "${title}" from favorites?`}
      description={`"${title}" is no longer in "${sourceName}". Its files will be deleted at the next sync.`}
      confirmLabel="Remove"
      pendingLabel="Removing…"
      onConfirm={onConfirm}
    />
  )
}
