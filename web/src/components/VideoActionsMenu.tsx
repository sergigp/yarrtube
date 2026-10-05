import { EllipsisVertical, Check } from 'lucide-react'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { cn } from '@/lib/utils'

interface VideoActionsMenuProps {
  videoId: string
  /** Names the video in the trigger's label and the failure alert. */
  title: string
  /** When false, "Mark as watched" is disabled. */
  markable: boolean
  className?: string
}

/** A vertical "⋮" menu of actions on one video. */
export function VideoActionsMenu({ title, className }: VideoActionsMenuProps) {
  return (
    <DropdownMenu modal={false}>
      <DropdownMenuTrigger
        className={cn(
          'flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground data-[state=open]:bg-secondary data-[state=open]:text-foreground',
          className,
        )}
        aria-label={`Actions for ${title}`}
      >
        <EllipsisVertical className="size-4" />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-auto min-w-44">
        <DropdownMenuItem>
          <Check />
          Mark as watched
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
