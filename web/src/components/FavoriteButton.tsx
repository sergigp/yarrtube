import { useState } from 'react'
import { Star } from 'lucide-react'
import { useSetVideoFavorite } from '@/api/queries'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { UnfavoriteKeptDialog } from './UnfavoriteKeptDialog'

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
 * showing the new state as soon as the server took it. Unfavoriting a kept
 * video asks first.
 */
export function FavoriteButton({ videoId, title, favorite, kept }: FavoriteButtonProps) {
  const setVideoFavorite = useSetVideoFavorite()
  const [shown, setShown] = useState(favorite)
  const [previous, setPrevious] = useState(favorite)
  const [confirming, setConfirming] = useState(false)
  if (previous !== favorite) {
    setPrevious(favorite)
    setShown(favorite)
  }
  const label = shown ? 'Remove from favorites' : 'Add to favorites'
  const toggle = async () => {
    await setVideoFavorite(videoId, !shown)
    setShown(!shown)
  }

  return (
    <>
      <Button
        variant="secondary"
        size="icon"
        className="size-9 rounded-full"
        aria-label={label}
        aria-pressed={shown}
        title={label}
        onClick={() => (shown && kept ? setConfirming(true) : toggle())}
      >
        <Star className={cn(shown && 'fill-current text-amber-500')} />
      </Button>
      {kept && (
        <UnfavoriteKeptDialog
          open={confirming}
          title={title}
          sourceName={kept.sourceName}
          onConfirm={toggle}
          onCancel={() => setConfirming(false)}
        />
      )}
    </>
  )
}
