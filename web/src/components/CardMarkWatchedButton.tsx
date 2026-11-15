import { useState } from 'react'
import { Check } from 'lucide-react'
import { useMarkVideoWatched } from '@/api/queries'
import { errorMessage } from '@/lib/errorMessage'

interface CardMarkWatchedButtonProps {
  videoId: string
  /** Names the video in the button's label and the failure alert. */
  title: string
}

/**
 * A video card's "Mark watched" quick action, over its thumbnail's top-right
 * corner. It shows while the card is hovered or the button keyboard focused,
 * and only where the device can hover: touch keeps it in the card's menu.
 * Its card (`group/card`) must be positioned above the stretched video link.
 */
export function CardMarkWatchedButton({ videoId, title }: CardMarkWatchedButtonProps) {
  const markVideoWatched = useMarkVideoWatched()
  const [marking, setMarking] = useState(false)

  return (
    <button
      type="button"
      aria-label={`Mark watched: ${title}`}
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
      className="absolute top-2 right-2 z-10 hidden h-8 items-center gap-1.5 rounded-lg bg-black/80 px-2.5 text-xs font-semibold text-white opacity-0 transition-opacity group-hover/card:opacity-100 hover:bg-black focus-visible:opacity-100 disabled:opacity-60 [@media(hover:hover)]:flex"
    >
      <Check aria-hidden="true" className="size-3.5" strokeWidth={2.5} />
      Mark watched
    </button>
  )
}
