import { Play } from 'lucide-react'
import { Link } from 'react-router-dom'
import { Button } from '@/components/ui/button'

/**
 * A page's main call to action, playing one video: "Play next · Ep 2" and,
 * from `sm` up, the video's title after it.
 */
export function PlayLink({
  to,
  label,
  title,
  compact = false,
}: {
  to: string
  label: string
  title?: string
  compact?: boolean
}) {
  return (
    <Button
      asChild
      className={
        compact
          ? 'h-9 max-w-md rounded-full px-3.5 font-semibold'
          : 'h-11 max-w-md rounded-full px-4 font-semibold md:h-10'
      }
    >
      <Link to={to} className="no-underline">
        <Play className="fill-current" />
        <span className="shrink-0">{label}</span>
        {title && (
          <span className="truncate font-normal opacity-85 max-sm:hidden">· {title}</span>
        )}
      </Link>
    </Button>
  )
}
