import { useState } from 'react'
import { Check } from 'lucide-react'
import { useMarkVideoWatched } from '@/api/queries'
import { Button } from '@/components/ui/button'
import { errorMessage } from '@/lib/errorMessage'
import { cn } from '@/lib/utils'

interface MarkWatchedButtonProps {
  videoId: string
  /** Names the video in the failure alert. */
  title: string
  watched: boolean
  downloaded: boolean
  className?: string
}

/**
 * The selected video's "Mark watched" button, grouped with its other
 * actions right of the title. Shown only while the video is downloaded and
 * unwatched; the meta line says when it is watched.
 */
export function MarkWatchedButton({
  videoId,
  title,
  watched,
  downloaded,
  className,
}: MarkWatchedButtonProps) {
  const markVideoWatched = useMarkVideoWatched()
  const [marking, setMarking] = useState(false)

  if (!downloaded || watched) {
    return null
  }

  return (
    <Button
      variant="secondary"
      className={cn('h-9 rounded-full px-3.5 font-semibold', className)}
      disabled={marking}
      onClick={async () => {
        setMarking(true)
        try {
          await markVideoWatched(videoId)
        } catch (err) {
          window.alert(`Failed to mark "${title}" watched: ${errorMessage(err)}`)
        } finally {
          setMarking(false)
        }
      }}
    >
      <Check />
      Mark watched
    </Button>
  )
}
