import { useDiscardVideo } from '@/api/queries'
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
}

/** The confirmation that discards a video once accepted. */
export function DiscardVideoDialog({
  videoId,
  title,
  downloaded,
  open,
  onOpenChange,
  onDiscarded,
}: DiscardVideoDialogProps) {
  const discardVideo = useDiscardVideo()

  return (
    <ConfirmDialog
      open={open}
      onOpenChange={onOpenChange}
      title={`Don't download "${title}"?`}
      description={
        downloaded
          ? "Its files will be deleted and it will never be downloaded again. This can't be undone."
          : "It will never be downloaded. This can't be undone."
      }
      confirmLabel="Discard video"
      pendingLabel="Discarding…"
      onConfirm={async () => {
        await discardVideo(videoId)
        onDiscarded?.()
      }}
    />
  )
}
