import { useCallback, useEffect, useId, useRef, useState } from 'react'
import { Link, useLocation, useNavigate, useParams, useSearchParams } from 'react-router-dom'
import { FolderPlus, Palette, Plus, Settings2 } from 'lucide-react'
import { deleteCategory, updateCategory } from '@/api/client'
import {
  useCategories,
  useCategoryVideos,
  useLibraryAction,
  useMarkCategoryWatched,
  usePreferences,
} from '@/api/queries'
import type { Category } from '@/api/types'
import { useTheme } from '@/hooks/useTheme'
import { deleteCategoryDescription, memberCount } from '@/lib/categories'
import { softTint } from '@/lib/categoryColors'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog'
import { CategoryColorPicker } from './CategoryColorPicker'
import { CategoryPlayer } from './CategoryPlayer'
import { DetailHeader } from './DetailHeader'
import { ManageSourcesDialog } from './ManageSourcesDialog'
import { VideoGrid } from './VideoGrid'
import { VideoGridSkeleton } from './VideoSkeletons'

/** Router state that opens the manage sources dialog on arrival at a category's view. */
export interface ManageSourcesState {
  manageSources: true
}

/**
 * What an empty category shows instead of videos: what it is for, and the
 * way to fill it, the manage sources dialog.
 */
function EmptyCategory({ category, onManage }: { category: Category; onManage: () => void }) {
  const headingId = useId()
  return (
    <section
      aria-labelledby={headingId}
      className="flex flex-col items-center gap-4 rounded-xl border border-dashed border-border px-6 py-16 text-center"
    >
      <span className="flex size-12 items-center justify-center rounded-full bg-secondary text-muted-foreground">
        <FolderPlus className="size-6" />
      </span>
      <div className="flex max-w-sm flex-col gap-1.5">
        <h2 id={headingId} className="font-heading text-lg font-semibold">
          Nothing in {category.name} yet
        </h2>
        <p className="text-sm text-muted-foreground">
          Add channels and playlists to {category.name} to watch their videos here, newest first.
        </p>
      </div>
      <Button type="button" onClick={onManage}>
        <Plus />
        Add channels and playlists
      </Button>
    </section>
  )
}

/** How long the chosen color must stay put before it is saved. */
const COLOR_SAVE_DELAY_MS = 300

/**
 * Saves the latest of a burst of choices (a hue slider drag) once it settles,
 * or at once on `flush` and unmount. Saves run one at a time, in order, so
 * the last choice is the one stored.
 */
function useSettledSave(save: (color: string) => Promise<void>) {
  const saveRef = useRef(save)
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
  const pending = useRef<string | null>(null)
  const queue = useRef<Promise<void>>(Promise.resolve())
  useEffect(() => {
    saveRef.current = save
  })
  const flush = useCallback(() => {
    clearTimeout(timer.current)
    const color = pending.current
    pending.current = null
    if (color !== null) {
      queue.current = queue.current.then(() => saveRef.current(color)).catch(() => {})
    }
  }, [])
  useEffect(() => flush, [flush])
  const schedule = (color: string) => {
    pending.current = color
    clearTimeout(timer.current)
    timer.current = setTimeout(flush, COLOR_SAVE_DELAY_MS)
  }
  return { schedule, flush }
}

/**
 * The category color picker in a dialog. The header follows each choice at
 * once (`onDraft`), and the choice is saved once it settles (see
 * `useSettledSave`) or the dialog closes.
 */
function CategoryColorDialog({
  open,
  onOpenChange,
  color,
  onDraft,
  onChange,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
  color: string
  onDraft: (color: string | null) => void
  onChange: (color: string) => Promise<void>
}) {
  const [draft, setDraft] = useState<string | null>(null)
  const { schedule, flush } = useSettledSave(onChange)
  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) {
          flush()
          setDraft(null)
          onDraft(null)
        }
        onOpenChange(next)
      }}
    >
      <DialogContent className="w-auto">
        <DialogTitle>Change color</DialogTitle>
        <DialogDescription className="sr-only">
          The category's color, saved as soon as it is chosen.
        </DialogDescription>
        <CategoryColorPicker
          value={draft ?? color}
          onChange={(next) => {
            setDraft(next)
            onDraft(next)
            schedule(next)
          }}
        />
      </DialogContent>
    </Dialog>
  )
}

/**
 * A category's view (`/categories/:id`): its header and videos, as a grid,
 * or, once one is picked (`?video=`), playing it with the category's videos
 * as the queue beside the player.
 */
export function CategoryDetail() {
  const { id = '' } = useParams()
  const [searchParams] = useSearchParams()
  const playing = searchParams.has('video')
  const navigate = useNavigate()
  const location = useLocation()
  const refreshing = useLibraryAction()
  const markCategoryWatched = useMarkCategoryWatched()
  const theme = useTheme()
  const [managing, setManaging] = useState(false)
  // Arriving with ManageSourcesState opens the dialog once per navigation;
  // the state is then dropped from history so reload and back don't reopen it.
  const arrivedManaging = (location.state as ManageSourcesState | null)?.manageSources === true
  const [openedFor, setOpenedFor] = useState<string | null>(null)
  if (arrivedManaging && openedFor !== location.key) {
    setOpenedFor(location.key)
    setManaging(true)
  }
  useEffect(() => {
    if (arrivedManaging) {
      navigate(location.pathname, { replace: true, state: null })
    }
  }, [arrivedManaging, location.pathname, navigate])
  const [recoloring, setRecoloring] = useState(false)
  const [draftColor, setDraftColor] = useState<string | null>(null)
  const { data } = useCategories()
  const { data: videos, error } = useCategoryVideos(id)
  const { data: preferences } = usePreferences()
  const category = data?.categories.find((candidate) => candidate.id === id)

  if (preferences && !preferences.categories_enabled) {
    return (
      <p className="text-sm text-muted-foreground">
        Categories are turned off.{' '}
        <Link to="/preferences" className="underline hover:text-foreground">
          Preferences
        </Link>
      </p>
    )
  }

  if (!data) {
    return <VideoGridSkeleton count={12} />
  }
  if (!category) {
    return <p className="text-sm text-muted-foreground">Category not found.</p>
  }
  const sources = memberCount(category)

  return (
    <div className={playing ? 'flex flex-col md:h-full md:min-h-[480px]' : 'flex min-w-0 flex-col'}>
      <DetailHeader
        name={category.name}
        tint={softTint(draftColor ?? category.color, theme)}
        videos={videos}
        unwatchedCount={category.unwatched_count}
        summaryPrefix={`${sources} ${sources === 1 ? 'source' : 'sources'}`}
        onRename={refreshing((name: string) => updateCategory(category.id, { name }))}
        extraActions={[
          { label: 'Change color', icon: <Palette />, onSelect: () => setRecoloring(true) },
          { label: 'Manage sources', icon: <Settings2 />, onSelect: () => setManaging(true) },
        ]}
        onMarkWatched={sources > 0 ? () => markCategoryWatched(category.id) : undefined}
        onDelete={refreshing(async () => {
          await deleteCategory(category.id)
          navigate('/')
        })}
        deleteDescription={deleteCategoryDescription(category)}
      />
      {sources === 0 ? (
        <EmptyCategory category={category} onManage={() => setManaging(true)} />
      ) : playing ? (
        <CategoryPlayer key={category.id} videos={videos} error={error} />
      ) : !videos ? (
        !error && <VideoGridSkeleton count={12} />
      ) : (
        videos && (
          <VideoGrid
            videos={videos}
            videoPathOf={(video) =>
              `/categories/${encodeURIComponent(category.id)}?video=${encodeURIComponent(video.id)}`
            }
          />
        )
      )}
      <ManageSourcesDialog category={category} open={managing} onOpenChange={setManaging} />
      <CategoryColorDialog
        open={recoloring}
        onOpenChange={setRecoloring}
        color={category.color}
        onDraft={setDraftColor}
        onChange={refreshing((color: string) => updateCategory(category.id, { color }))}
      />
    </div>
  )
}
