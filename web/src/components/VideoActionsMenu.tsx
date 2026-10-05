import { EllipsisVertical, Check } from 'lucide-react'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { useMarkVideoWatched } from '@/api/queries'
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
export function VideoActionsMenu({ videoId, title, markable, className }: VideoActionsMenuProps) {
  const markVideoWatched = useMarkVideoWatched()

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
        <DropdownMenuItem
          disabled={!markable}
          onSelect={async () => {
            try {
              await markVideoWatched(videoId)
            } catch (err) {
              window.alert(`Failed to mark "${title}" watched: ${errorMessage(err)}`)
            }
          }}
        >
          <Check />
          Mark as watched
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

function errorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err)
}
