import type { ReactNode } from 'react'
import { CheckCheck, EllipsisVertical, Eye, EyeOff, Trash2 } from 'lucide-react'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { errorMessage } from '@/lib/errorMessage'
import { homeSettingAction } from '@/lib/homeSetting'
import { cn } from '@/lib/utils'

interface EntryActionsMenuProps {
  /** Names the channel or playlist in the trigger's label and failure alerts. */
  name: string
  /** Items listed first, e.g. the sidebar's Sync; they report their own failures. */
  leadingItems?: ReactNode
  /** Replaces the trigger's default vertical "⋮" icon. */
  triggerIcon?: ReactNode
  /** Channels only: marks every downloaded video watched. */
  onMarkWatched?: (() => Promise<void>) | undefined
  /** Playlists only: the current setting; undefined hides the home item. */
  excludedFromHome?: boolean | undefined
  onSetExcludedFromHome?: ((excluded: boolean) => Promise<void>) | undefined
  /** Asks for confirmation; the caller owns the confirm dialog. */
  onDeleteRequest: () => void
  className?: string
}

/**
 * A "⋮" menu of actions on one channel or playlist, shared by its sidebar row
 * and its page header. It alerts when marking watched or changing the home
 * setting fails, leaving the entry as it was.
 */
export function EntryActionsMenu({
  name,
  leadingItems,
  triggerIcon,
  onMarkWatched,
  excludedFromHome,
  onSetExcludedFromHome,
  onDeleteRequest,
  className,
}: EntryActionsMenuProps) {
  const offersHomeItem = excludedFromHome !== undefined && onSetExcludedFromHome !== undefined
  const hasItemsAboveDelete = Boolean(leadingItems) || Boolean(onMarkWatched) || offersHomeItem

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
        {triggerIcon ?? <EllipsisVertical className="size-4" />}
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-auto min-w-44">
        {leadingItems}
        {onMarkWatched && (
          <DropdownMenuItem
            onSelect={async () => {
              try {
                await onMarkWatched()
              } catch (err) {
                window.alert(`Failed to mark "${name}" watched: ${errorMessage(err)}`)
              }
            }}
          >
            <CheckCheck />
            Mark all watched
          </DropdownMenuItem>
        )}
        {offersHomeItem && (
          <DropdownMenuItem
            onSelect={async () => {
              try {
                await onSetExcludedFromHome(!excludedFromHome)
              } catch (err) {
                window.alert(
                  `Failed to ${homeSettingAction(!excludedFromHome, name)}: ${errorMessage(err)}`,
                )
              }
            }}
          >
            {excludedFromHome ? <Eye /> : <EyeOff />}
            {excludedFromHome ? 'Include in home' : 'Exclude from home'}
          </DropdownMenuItem>
        )}
        {hasItemsAboveDelete && <DropdownMenuSeparator />}
        <DropdownMenuItem variant="destructive" onSelect={onDeleteRequest}>
          <Trash2 />
          Delete
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
