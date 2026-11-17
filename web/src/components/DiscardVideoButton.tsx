import { useState } from 'react'
import { Ban } from 'lucide-react'
import { useDiscardVideo } from '@/api/queries'
import { Button } from '@/components/ui/button'
import { ConfirmDialog } from './ConfirmDialog'

interface DiscardVideoButtonProps {
  videoId: string
  /** Names the video in the confirmation. */
  title: string
  /** Whether the video has files that discarding deletes. */
  downloaded: boolean
}

/**
 * The video detail pane's "Don't download" button: after a confirmation
 * that it can't be undone, discards the video for good.
 */
export function DiscardVideoButton({ videoId, title, downloaded }: DiscardVideoButtonProps) {
  const discardVideo = useDiscardVideo()
  const [confirming, setConfirming] = useState(false)

  return (
    <>
      <Button variant="outline" size="sm" className="mb-2" onClick={() => setConfirming(true)}>
        <Ban />
        Don't download
      </Button>
      <ConfirmDialog
        open={confirming}
        onOpenChange={setConfirming}
        title={`Don't download "${title}"?`}
        description={
          downloaded
            ? "Its files will be deleted and it will never be downloaded again. This can't be undone."
            : "It will never be downloaded. This can't be undone."
        }
        confirmLabel="Discard video"
        pendingLabel="Discarding…"
        onConfirm={() => discardVideo(videoId)}
      />
    </>
  )
}
