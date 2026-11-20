import { useState } from 'react'
import { Ban } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { DiscardVideoDialog } from './DiscardVideoDialog'

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
  const [confirming, setConfirming] = useState(false)

  return (
    <>
      <Button variant="outline" size="sm" className="mb-2" onClick={() => setConfirming(true)}>
        <Ban />
        Don't download
      </Button>
      <DiscardVideoDialog
        videoId={videoId}
        title={title}
        downloaded={downloaded}
        open={confirming}
        onOpenChange={setConfirming}
      />
    </>
  )
}
