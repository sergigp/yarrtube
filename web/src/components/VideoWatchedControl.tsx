import { useState } from 'react'
import { Check } from 'lucide-react'
import { useMarkVideoWatched } from '@/api/queries'
import { Button } from '@/components/ui/button'
import { errorMessage } from '@/lib/errorMessage'
import { cn } from '@/lib/utils'

interface VideoWatchedControlProps {
  videoId: string
  /** Names the video in the failure alert. */
  title: string
  watched: boolean
  downloaded: boolean
  className?: string
}

/**
 * The selected video's labelled watched control, on a line of its own in the
 * video detail pane: "Mark watched" while unwatched, a "Watched" status once
 * watched, and nothing until the video has downloaded.
 */
export function VideoWatchedControl({
  videoId,
  title,
  watched,
  downloaded,
  className,
}: VideoWatchedControlProps) {
  const markVideoWatched = useMarkVideoWatched()
  const [marking, setMarking] = useState(false)

  if (!downloaded) {
    return null
  }
  if (watched) {
    // Not a button: watched comes back off only by rewatching, as the hint says.
    return (
      <span
        className={cn(
          'flex h-7 w-fit items-center gap-1 text-[0.8rem] font-medium text-muted-foreground',
          className,
        )}
        title="Rewatch past 10% to mark it unwatched"
      >
        <Check className="size-3.5" />
        Watched
      </span>
    )
  }

  return (
    <Button
      variant="outline"
      size="sm"
      className={cn('flex w-fit', className)}
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
