import type { FilterChip, TaskFilter } from '@/lib/tasks'
import { cn } from '@/lib/utils'

const CHIP_LABELS: Record<TaskFilter, string> = {
  all: 'All',
  downloads: 'Downloads',
  syncs: 'Syncs',
  ai: 'AI',
  upkeep: 'Plex & upkeep',
}

interface TaskFilterChipsProps {
  chips: FilterChip[]
  selected: TaskFilter
  onSelect: (filter: TaskFilter) => void
}

/** The family filters, each with its task count, scrolling sideways when they don't fit. */
export function TaskFilterChips({ chips, selected, onSelect }: TaskFilterChipsProps) {
  return (
    <div className="-mx-4 flex gap-2 overflow-x-auto px-4 pb-1 sm:mx-0 sm:flex-wrap sm:px-0">
      {chips.map(({ filter, count }) => {
        const pressed = filter === selected
        return (
          <button
            key={filter}
            type="button"
            aria-pressed={pressed}
            onClick={() => onSelect(filter)}
            className={cn(
              'inline-flex min-h-11 shrink-0 items-center gap-2 rounded-full border px-4 text-sm font-medium whitespace-nowrap transition-colors outline-none focus-visible:ring-3 focus-visible:ring-ring/50 sm:min-h-8 sm:px-3',
              pressed
                ? 'border-primary bg-accent text-foreground'
                : 'border-border text-muted-foreground hover:text-foreground',
            )}
          >
            <span>{CHIP_LABELS[filter]}</span>
            <span className="text-xs tabular-nums">{count}</span>
          </button>
        )
      })}
    </div>
  )
}
