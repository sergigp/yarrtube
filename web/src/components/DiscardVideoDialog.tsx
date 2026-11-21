import { useDiscardVideo } from '@/api/queries'
import { errorMessage } from '@/lib/errorMessage'
import { ConfirmDialog } from './ConfirmDialog'

interface DiscardVideoDialogProps {
  videoId: string
  /** Names the video in the confirmation. */
  title: string
  /** Whether the video has files that discarding deletes. */
  downloaded: boolean
  open: boolean
  onOpenChange: (open: boolean) => void
  /** Called after a successful discard. */
  onDiscarded?: () => void
  /**
   * When set, a failed discard closes the dialog and is reported here
   * instead of inside the dialog.
   */
  onFailed?: (message: string) => void
}

/** The confirmation that discards a video once accepted. */
export function DiscardVideoDialog({
  videoId,
  title,
  downloaded,
  open,
  onOpenChange,
  onDiscarded,
  onFailed,
}: DiscardVideoDialogProps) {
  const discardVideo = useDiscardVideo()

  return (
    <ConfirmDialog
      open={open}
      onOpenChange={onOpenChange}
      title={`Discard "${title}"?`}
      description={
        downloaded
          ? "Its files will be deleted and it won't be downloaded. You can restore it from its channel's or playlist's menu."
          : "It won't be downloaded. You can restore it from its channel's or playlist's menu."
      }
      confirmLabel="Discard"
      pendingLabel="Discarding…"
      onConfirm={async () => {
        try {
          await discardVideo(videoId)
        } catch (err) {
          if (!onFailed) throw err
          onOpenChange(false)
          onFailed(errorMessage(err))
          return
        }
        onDiscarded?.()
      }}
    />
  )
}
