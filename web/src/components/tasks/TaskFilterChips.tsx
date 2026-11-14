import type { FilterChip, TaskFilter } from '@/lib/tasks'

interface TaskFilterChipsProps {
  chips: FilterChip[]
  selected: TaskFilter
  onSelect: (filter: TaskFilter) => void
}

/** The family filters, each with its task count. */
export function TaskFilterChips(_props: TaskFilterChipsProps) {
  return null
}
