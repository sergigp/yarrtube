import { ListVideo } from 'lucide-react'
import { cn } from '@/lib/utils'

/** Stands in for a playlist's avatar (sidebar rows, card meta lines), so playlists read differently from channels. */
export function PlaylistIcon({ className }: { className?: string }) {
  return (
    <span
      aria-hidden="true"
      className={cn(
        'flex size-5 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground',
        className,
      )}
    >
      <ListVideo className="size-3" />
    </span>
  )
}
