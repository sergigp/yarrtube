import { ChevronDown } from 'lucide-react'
import { cn } from '@/lib/utils'

/** A sidebar row (`li`): the hover surface, and the group its menu control shows on. */
export const SIDEBAR_ROW_CLASS =
  'group/row relative flex items-center rounded-md transition-colors hover:bg-secondary'

/** A sidebar row's link: 32px tall, its icon, name and count spaced alike in every section. */
export function sidebarRowLinkClassName(active: boolean): string {
  return cn(
    'flex h-8 min-w-0 flex-1 items-center gap-3 px-3 text-sm no-underline',
    active ? 'font-medium text-primary' : 'text-foreground',
  )
}

/** Below a section's first rows: reveals the rest. */
export function ShowMoreButton({ count, onClick }: { count: number; onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="flex h-8 w-full items-center gap-3 rounded-md px-3 text-sm text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground"
    >
      <ChevronDown aria-hidden="true" className="size-4 shrink-0" />
      Show {count} more
    </button>
  )
}
