import { Check } from 'lucide-react'

/**
 * A tick overlaid on a video thumbnail when that video has been watched.
 * Its parent must be positioned (`relative`).
 */
export function WatchedTick({ watched }) {
  if (!watched) {
    return null
  }

  return (
    <span
      className="absolute bottom-1 left-1 flex size-5 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-sm"
      role="img"
      aria-label="Watched"
      title="Watched"
    >
      <Check className="size-3.5" strokeWidth={3} />
    </span>
  )
}
