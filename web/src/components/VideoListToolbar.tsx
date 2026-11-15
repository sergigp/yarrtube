import type { ReactNode } from 'react'
import { ArrowDownUp, ChevronDown } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import {
  VIDEO_SORT_LABELS,
  WATCH_FILTERS,
  WATCH_FILTER_LABELS,
  type VideoSort,
  type WatchFilter,
} from '@/lib/sourceVideos'
import { cn } from '@/lib/utils'

interface VideoListToolbarProps {
  counts: Record<WatchFilter, number>
  filter: WatchFilter
  onFilterChange: (filter: WatchFilter) => void
  sorts: readonly VideoSort[]
  sort: VideoSort
  onSortChange: (sort: VideoSort) => void
  /** Shown before the sort, e.g. a category's source picker. */
  children?: ReactNode
}

/**
 * One line over a page's video grid: the watch-state chips with their counts
 * (scrolling sideways on phones), and on the right anything extra plus the
 * sort.
 */
export function VideoListToolbar({
  counts,
  filter,
  onFilterChange,
  sorts,
  sort,
  onSortChange,
  children,
}: VideoListToolbarProps) {
  return (
    <div className="mb-5 flex flex-wrap items-center justify-between gap-3 border-b border-border pb-4 md:mb-6">
      <div
        role="group"
        aria-label="Show"
        className="-mx-4 flex max-w-[calc(100%+2rem)] gap-2 overflow-x-auto px-4 sm:mx-0 sm:max-w-full sm:flex-wrap sm:px-0"
      >
        {WATCH_FILTERS.map((option) => {
          const pressed = option === filter
          return (
            <button
              key={option}
              type="button"
              aria-pressed={pressed}
              onClick={() => onFilterChange(option)}
              className={cn(
                'inline-flex h-8 shrink-0 items-center gap-1.5 rounded-lg px-3 text-sm font-semibold whitespace-nowrap transition-colors outline-none focus-visible:ring-3 focus-visible:ring-ring/50',
                pressed
                  ? 'bg-foreground text-background'
                  : 'bg-secondary text-secondary-foreground hover:bg-accent',
              )}
            >
              {WATCH_FILTER_LABELS[option]}
              <span className="font-normal tabular-nums opacity-70">{counts[option]}</span>
            </button>
          )
        })}
      </div>
      <div className="flex min-w-0 items-center gap-2">
        {children}
        <ChoiceMenu
          label="Sort"
          icon={<ArrowDownUp />}
          value={sort}
          choices={sorts.map((value) => ({ value, label: VIDEO_SORT_LABELS[value] }))}
          onChange={onSortChange}
        />
      </div>
    </div>
  )
}

/**
 * A toolbar button showing the current choice, opening a menu of the
 * others. `label` names it for assistive technology along with the choice
 * ("Sort: Newest first").
 */
export function ChoiceMenu<T extends string>({
  label,
  icon,
  prefix,
  value,
  choices,
  onChange,
}: {
  label: string
  icon?: ReactNode
  /** Shown before the choice on the button, e.g. "From:". */
  prefix?: string
  value: T
  choices: { value: T; label: string; icon?: ReactNode; count?: number }[]
  onChange: (value: T) => void
}) {
  const current = choices.find((choice) => choice.value === value)?.label ?? ''
  return (
    <DropdownMenu modal={false}>
      <DropdownMenuTrigger asChild>
        <Button
          variant="outline"
          className="h-8 min-w-0 rounded-lg px-3 font-normal"
          aria-label={`${label}: ${current}`}
        >
          {icon}
          <span className="truncate">
            {prefix && `${prefix} `}
            {current}
          </span>
          {!icon && <ChevronDown />}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="max-h-80 w-auto max-w-80 min-w-44">
        <DropdownMenuRadioGroup value={value} onValueChange={(next) => onChange(next as T)}>
          {choices.map((choice) => (
            <DropdownMenuRadioItem key={choice.value} value={choice.value} className="gap-2">
              {choice.icon}
              <span className="min-w-0 flex-1 truncate">{choice.label}</span>
              {choice.count !== undefined && (
                <span className="text-xs text-muted-foreground tabular-nums">{choice.count}</span>
              )}
            </DropdownMenuRadioItem>
          ))}
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

