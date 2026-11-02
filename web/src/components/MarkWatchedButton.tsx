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
 * The selected video's "Mark watched" button in the video detail pane's
 * title row: labelled from the desktop breakpoint, a bare tick below it so
 * the title keeps room. Shown only while the video is downloaded and
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
      variant="outline"
      size="sm"
      aria-label="Mark watched"
      className={cn('max-md:w-7 max-md:px-0', className)}
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
      <span className="max-md:hidden">Mark watched</span>
    </Button>
  )
}
