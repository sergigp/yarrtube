import { ChevronDown, ChevronRight, Plus } from 'lucide-react'
import { cn } from '@/lib/utils'

interface SidebarSectionHeadingProps {
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
  title,
  folded,
  onToggleFolded,
  headingActive,
  foldedTotal,
  add,
}: SidebarSectionHeadingProps) {
  return (
    <div className="mb-1 flex items-center gap-1">
      <h3 className="flex min-w-0 flex-1 font-heading text-base font-semibold tracking-tight text-foreground">
        <button
          type="button"
          className={cn(
            'flex w-full min-w-0 items-center gap-1 rounded-md px-2 py-1 text-left transition-colors hover:text-primary',
            headingActive && 'bg-accent text-primary',
          )}
          aria-expanded={!folded}
          aria-current={headingActive || undefined}
          onClick={onToggleFolded}
        >
          {folded ? (
            <ChevronRight className="size-4 shrink-0" />
          ) : (
            <ChevronDown className="size-4 shrink-0" />
          )}
          {title}
          {folded && foldedTotal !== undefined && foldedTotal > 0 && (
            <span
              className="ml-auto shrink-0 rounded-full bg-primary px-1.5 py-0.5 text-[10px] leading-none font-medium text-primary-foreground"
              aria-label={`${foldedTotal} unwatched`}
              title={`${foldedTotal} unwatched`}
            >
              {foldedTotal}
            </span>
          )}
        </button>
      </h3>
      {add && !folded && (
        <button
          type="button"
          className="shrink-0 rounded-md p-1 text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground"
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
