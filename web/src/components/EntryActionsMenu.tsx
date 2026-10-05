import { EllipsisVertical, Trash2 } from 'lucide-react'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { cn } from '@/lib/utils'

interface EntryActionsMenuProps {
  /** Names the channel or playlist in the trigger's label and failure alerts. */
  name: string
  /** Channels only: marks every downloaded video watched. */
  onMarkWatched?: () => Promise<void>
  /** Playlists only: the current setting; undefined hides the home item. */
  excludedFromHome?: boolean
  onSetExcludedFromHome?: (excluded: boolean) => Promise<void>
  /** Asks for confirmation; the caller owns the confirm dialog. */
  onDeleteRequest: () => void
  className?: string
}

/** A vertical "⋮" menu of actions on one channel or playlist. */
export function EntryActionsMenu({ name, onDeleteRequest, className }: EntryActionsMenuProps) {
  return (
    // Non-modal so opening the delete confirmation from it doesn't leave the
    // page with pointer events disabled.
    <DropdownMenu modal={false}>
      <DropdownMenuTrigger
        className={cn(
          'flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground data-[state=open]:bg-secondary data-[state=open]:text-foreground',
          className,
        )}
        aria-label={`Actions for ${name}`}
      >
        <EllipsisVertical className="size-4" />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-auto min-w-44">
        <DropdownMenuItem variant="destructive" onSelect={onDeleteRequest}>
          <Trash2 />
          Delete
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
