import { useState } from 'react'
import { Check } from 'lucide-react'
import { useMarkVideoWatched } from '@/api/queries'
import { Button } from '@/components/ui/button'
import { errorMessage } from '@/lib/errorMessage'

interface VideoWatchedControlProps {
  videoId: string
  /** Names the video in the failure alert. */
  title: string
  watched: boolean
  downloaded: boolean
}

/** The selected video's labelled watched control, in the video detail pane. */
export function VideoWatchedControl({
  videoId,
  title,
  watched,
  downloaded,
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
        className="inline-flex h-7 items-center gap-1 text-[0.8rem] font-medium text-muted-foreground"
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
