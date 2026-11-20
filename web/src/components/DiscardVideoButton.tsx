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
 * The video detail pane's "Discard" button: after a confirmation, discards
 * the video until its channel or playlist restores it.
 */
export function DiscardVideoButton({ videoId, title, downloaded }: DiscardVideoButtonProps) {
  const [confirming, setConfirming] = useState(false)

  return (
    <>
      <Button variant="outline" size="sm" className="mb-2" onClick={() => setConfirming(true)}>
        <Ban />
        Discard
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
