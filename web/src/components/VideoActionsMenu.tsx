import { EllipsisVertical, Check, EyeOff, Tags } from 'lucide-react'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import {
  useCategoriesShown,
  useMarkVideoWatched,
  useSetPlaylistExcludedFromHome,
} from '@/api/queries'
import { useEditVideoCategories } from '@/hooks/useEditVideoCategories'
import { errorMessage } from '@/lib/errorMessage'
import { homeSettingAction } from '@/lib/homeSetting'
import { cn } from '@/lib/utils'

interface VideoActionsMenuProps {
  /** The YouTube video id. */
  videoId: string
  /** The stored copy's `record_id`, naming it to the categories routes. */
  recordId: string
  /** Names the video in the trigger's label and the failure alert. */
  title: string
  /** The channel or playlist the copy comes from. */
  sourceName: string
  /** When false, "Mark as watched" is disabled. */
  markable: boolean
  /** The playlist the card is sourced from, offered for exclusion from home. */
  excludablePlaylist?: { id: string; name: string }
  className?: string
}

/** A vertical "⋮" menu of actions on one video. */
export function VideoActionsMenu({
  videoId,
  recordId,
  title,
  sourceName,
  markable,
  excludablePlaylist,
  className,
}: VideoActionsMenuProps) {
  const markVideoWatched = useMarkVideoWatched()
  const setPlaylistExcludedFromHome = useSetPlaylistExcludedFromHome()
  const categoriesShown = useCategoriesShown()
  const editCategories = useEditVideoCategories()

  return (
    <DropdownMenu modal={false}>
      <DropdownMenuTrigger
        className={cn(
          // On touch, an invisible box 8px past each edge makes the hit area 44px.
          "relative pointer-coarse:after:absolute pointer-coarse:after:-inset-2 pointer-coarse:after:content-['']",
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
        {excludablePlaylist && (
          <DropdownMenuItem
            onSelect={async () => {
              try {
                await setPlaylistExcludedFromHome(excludablePlaylist.id, true)
              } catch (err) {
                window.alert(
                  `Failed to ${homeSettingAction(true, excludablePlaylist.name)}: ${errorMessage(err)}`,
                )
              }
            }}
          >
            <EyeOff />
            Exclude "{excludablePlaylist.name}" from home
          </DropdownMenuItem>
        )}
        {categoriesShown && (
          <DropdownMenuItem onSelect={() => editCategories({ recordId, title, sourceName })}>
            <Tags />
            Edit categories…
          </DropdownMenuItem>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
