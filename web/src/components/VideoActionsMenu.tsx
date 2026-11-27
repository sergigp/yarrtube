import { useState } from 'react'
import { Ban, EllipsisVertical, Check, EyeOff, Star, Tags } from 'lucide-react'
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
  useSetVideoFavorite,
} from '@/api/queries'
import { useEditVideoCategories } from '@/hooks/useEditVideoCategories'
import { errorMessage } from '@/lib/errorMessage'
import { homeSettingAction } from '@/lib/homeSetting'
import { cn } from '@/lib/utils'
import { DiscardVideoDialog } from './DiscardVideoDialog'
import { UnfavoriteKeptDialog } from './UnfavoriteKeptDialog'

interface VideoActionsMenuProps {
  /** The stored video's `id`. */
  videoId: string
  /** Names the video in the trigger's label and the failure alert. */
  title: string
  /** The channel or playlist the copy comes from. */
  sourceName: string
  /** When false, "Mark as watched" is disabled. */
  markable: boolean
  /** The playlist the card is sourced from, offered for exclusion from home. */
  excludablePlaylist?: { id: string; name: string }
  /** Offers "Discard…"; `downloaded` says whether discarding deletes its files. */
  discard?: { downloaded: boolean }
  /**
   * Offers "Add to favorites" or "Remove from favorites"; a `kept` video asks
   * before it is unfavorited.
   */
  favorite?: { favorite: boolean; kept?: { sourceName: string } }
  className?: string
}

/** A vertical "⋮" menu of actions on one video. */
export function VideoActionsMenu({
  videoId,
  title,
  sourceName,
  markable,
  excludablePlaylist,
  discard,
  favorite,
  className,
}: VideoActionsMenuProps) {
  const [confirmingDiscard, setConfirmingDiscard] = useState(false)
  const [confirmingUnfavorite, setConfirmingUnfavorite] = useState(false)
  const setVideoFavorite = useSetVideoFavorite()
  const markVideoWatched = useMarkVideoWatched()
  const setPlaylistExcludedFromHome = useSetPlaylistExcludedFromHome()
  const categoriesShown = useCategoriesShown()
  const editCategories = useEditVideoCategories()

  return (
    <>
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
          {favorite && (
            <DropdownMenuItem
              onSelect={async () => {
                if (favorite.favorite && favorite.kept) {
                  setConfirmingUnfavorite(true)
                  return
                }
                try {
                  await setVideoFavorite(videoId, !favorite.favorite)
                } catch (err) {
                  const action = favorite.favorite
                    ? `remove "${title}" from`
                    : `add "${title}" to`
                  window.alert(`Failed to ${action} favorites: ${errorMessage(err)}`)
                }
              }}
            >
              <Star />
              {favorite.favorite ? 'Remove from favorites' : 'Add to favorites'}
            </DropdownMenuItem>
          )}
          {categoriesShown && (
            <DropdownMenuItem onSelect={() => editCategories({ videoId, title, sourceName })}>
              <Tags />
              Edit categories…
            </DropdownMenuItem>
          )}
          {discard && (
            <DropdownMenuItem onSelect={() => setConfirmingDiscard(true)}>
              <Ban />
              Discard…
            </DropdownMenuItem>
          )}
        </DropdownMenuContent>
      </DropdownMenu>
      {discard && (
        <DiscardVideoDialog
          videoId={videoId}
          title={title}
          downloaded={discard.downloaded}
          open={confirmingDiscard}
          onOpenChange={setConfirmingDiscard}
        />
      )}
      {favorite?.kept && (
        <UnfavoriteKeptDialog
          open={confirmingUnfavorite}
          title={title}
          sourceName={favorite.kept.sourceName}
          onConfirm={() => setVideoFavorite(videoId, false)}
          onCancel={() => setConfirmingUnfavorite(false)}
        />
      )}
    </>
  )
}
