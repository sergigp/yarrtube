import { useState } from 'react'
import { Star } from 'lucide-react'
import { useSetVideoFavorite } from '@/api/queries'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'

interface FavoriteButtonProps {
  videoId: string
  /** Names the video in the unfavorite-kept confirmation. */
  title: string
  favorite: boolean
  /** Set when the video is kept by its source: unfavoriting asks first. */
  kept?: { sourceName: string }
}

/**
 * The video detail pane's star toggle: favorites or unfavorites the video,
 * showing the new state as soon as the server took it.
 */
export function FavoriteButton({ videoId, favorite }: FavoriteButtonProps) {
  const setVideoFavorite = useSetVideoFavorite()
  const [shown, setShown] = useState(favorite)
  const [previous, setPrevious] = useState(favorite)
  if (previous !== favorite) {
    setPrevious(favorite)
    setShown(favorite)
  }
  const label = shown ? 'Remove from favorites' : 'Add to favorites'

  return (
    <Button
      variant="secondary"
      size="icon"
      className="size-9 rounded-full"
      aria-label={label}
      aria-pressed={shown}
      title={label}
      onClick={async () => {
        await setVideoFavorite(videoId, !shown)
        setShown(!shown)
      }}
    >
      <Star className={cn(shown && 'fill-current text-amber-500')} />
    </Button>
  )
}
