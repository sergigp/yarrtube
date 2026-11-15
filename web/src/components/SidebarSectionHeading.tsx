import { ChevronDown, Plus } from 'lucide-react'
import { cn } from '@/lib/utils'
import { SidebarCount } from './SidebarCount'

interface SidebarSectionHeadingProps {
  /** The heading's id, for the section's list to be named after it. */
  id?: string | undefined
  title: string
  folded: boolean
  onToggleFolded: () => void
  headingActive: boolean
  /** Shown as an unwatched badge when folded and > 0. */
  foldedTotal?: number | undefined
  /** A compact add control at the heading's right, shown while unfolded. */
  add?: { label: string; onClick: () => void } | undefined
}

/** A sidebar section's heading: a fold toggle, plus an optional add control. */
export function SidebarSectionHeading({
  id,
  title,
  folded,
  onToggleFolded,
  headingActive,
  foldedTotal,
  add,
}: SidebarSectionHeadingProps) {
  return (
    <div className="flex h-7 items-center gap-1">
      <h3
        id={id}
        className="flex min-w-0 flex-1 text-xs font-semibold tracking-wider text-muted-foreground uppercase"
      >
        <button
          type="button"
          className={cn(
            'group/heading flex h-7 w-full min-w-0 items-center gap-1 rounded-md px-3 text-left uppercase transition-colors hover:text-foreground',
            headingActive && 'bg-accent text-primary',
          )}
          aria-expanded={!folded}
          aria-current={headingActive || undefined}
          onClick={onToggleFolded}
        >
          {title}
          {/* The fold control is quiet: its chevron shows while folded or pointed at. */}
          <ChevronDown
            aria-hidden="true"
            className={cn(
              'size-3.5 shrink-0 transition-[rotate,opacity]',
              folded
                ? '-rotate-90'
                : 'opacity-0 group-hover/heading:opacity-100 group-focus-visible/heading:opacity-100',
            )}
          />
          {folded && foldedTotal !== undefined && foldedTotal > 0 && (
            <SidebarCount count={foldedTotal} active={headingActive} className="ml-auto" />
          )}
        </button>
      </h3>
      {add && !folded && (
        <button
          type="button"
          className="flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground"
          aria-label={add.label}
          title={add.label}
          onClick={add.onClick}
        >
          <Plus className="size-4" />
        </button>
      )}
    </div>
  )
}
