import type { ReactNode } from 'react'
import { ChevronDown, ChevronRight } from 'lucide-react'
import { cn } from '@/lib/utils'

interface SidebarSectionHeadingProps {
  title: string
  folded: boolean
  onToggleFolded: () => void
  headingActive: boolean
  /** Shown, muted, when folded and > 0. */
  foldedTotal?: number | undefined
  /** Controls next to the fold toggle, e.g. a manage link; they don't fold. */
  actions?: ReactNode
}

/** A sidebar section's heading: a fold toggle, with optional trailing actions. */
export function SidebarSectionHeading({
  title,
  folded,
  onToggleFolded,
  headingActive,
  foldedTotal,
  actions,
}: SidebarSectionHeadingProps) {
  return (
    <h3 className="mb-1 flex items-center gap-1 font-heading text-base font-semibold tracking-tight text-foreground">
      <button
        type="button"
        className={cn(
          'flex w-full items-center gap-1 rounded-md px-2 py-1 text-left transition-colors hover:text-primary',
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
            className="ml-1 text-xs font-normal text-muted-foreground"
            aria-label={`${foldedTotal} unwatched`}
          >
            {foldedTotal}
          </span>
        )}
      </button>
      {actions}
    </h3>
  )
}
