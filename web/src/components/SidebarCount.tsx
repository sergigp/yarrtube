import { cn } from '@/lib/utils'

/**
 * A sidebar row's or folded heading's unwatched count: a small muted numeral
 * at the trailing edge, in the full foreground colour on the active row.
 */
export function SidebarCount({
  count,
  active = false,
  className,
}: {
  count: number
  active?: boolean
  className?: string
}) {
  return (
    <span
      className={cn(
        'shrink-0 text-xs font-normal tabular-nums',
        active ? 'text-foreground' : 'text-muted-foreground',
        className,
      )}
      aria-label={`${count} unwatched`}
      title={`${count} unwatched`}
    >
      {count}
    </span>
  )
}
