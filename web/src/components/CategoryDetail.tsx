import { useCallback, useEffect, useId, useRef, useState } from 'react'
import { useLocation, useNavigate, useParams } from 'react-router-dom'
import { FolderPlus, Plus, Settings2 } from 'lucide-react'
import { deleteCategory, updateCategory } from '@/api/client'
import {
  useCategories,
  useCategoryVideos,
  useLibraryAction,
  useMarkCategoryWatched,
} from '@/api/queries'
import type { Category } from '@/api/types'
import { deleteCategoryDescription, memberCount } from '@/lib/categories'
import { Button } from '@/components/ui/button'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { CategoryColorPicker } from './CategoryColorPicker'
import { DetailHeader } from './DetailHeader'
import { ManageSourcesDialog } from './ManageSourcesDialog'
import { VideoGrid } from './VideoGrid'

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
 * A dot in the category's color that opens the color picker. The picker
 * shows each choice at once and saves it once it settles (see `useSettledSave`).
 */
function CategoryColorControl({
  color,
  onChange,
}: {
  color: string
  onChange: (color: string) => Promise<void>
}) {
  const [draft, setDraft] = useState<string | null>(null)
  const { schedule, flush } = useSettledSave(onChange)
  return (
    <Popover
      onOpenChange={(open) => {
        if (!open) {
          flush()
          setDraft(null)
        }
      }}
    >
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>
            <PopoverTrigger asChild>
              <Button
                variant="outline"
                size="icon"
                aria-label="Change color"
                className="shrink-0 rounded-full"
              >
                <span
                  className="size-4 rounded-full border border-black/10"
                  style={{ background: draft ?? color }}
                />
              </Button>
            </PopoverTrigger>
          </TooltipTrigger>
          <TooltipContent>Change color</TooltipContent>
        </Tooltip>
      </TooltipProvider>
      <PopoverContent align="start" className="w-auto">
        <CategoryColorPicker
          value={draft ?? color}
          onChange={(next) => {
            setDraft(next)
            schedule(next)
          }}
        />
      </PopoverContent>
    </Popover>
  )
}

/** A category's view (`/categories/:id`): its header and videos. */
export function CategoryDetail() {
  const { id = '' } = useParams()
  const navigate = useNavigate()
  const location = useLocation()
  const refreshing = useLibraryAction()
  const markCategoryWatched = useMarkCategoryWatched()
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
  const { data } = useCategories()
  const { data: videos } = useCategoryVideos(id)
  const category = data?.categories.find((candidate) => candidate.id === id)

  if (!data) {
    return <p className="text-sm text-muted-foreground">Loading category…</p>
  }
  if (!category) {
    return <p className="text-sm text-muted-foreground">Category not found.</p>
  }
  const sources = memberCount(category)

  return (
    <div className="flex min-w-0 flex-col">
      <DetailHeader
        name={category.name}
        tint={category.color}
        colorControl={
          <CategoryColorControl
            color={category.color}
            onChange={refreshing((color: string) => updateCategory(category.id, { color }))}
          />
        }
        videos={videos}
        unwatchedCount={category.unwatched_count}
        summaryPrefix={`${sources} ${sources === 1 ? 'source' : 'sources'}`}
        onRename={refreshing((name: string) => updateCategory(category.id, { name }))}
        extraActions={[
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
      ) : (
        videos && <VideoGrid videos={videos} />
      )}
      <ManageSourcesDialog category={category} open={managing} onOpenChange={setManaging} />
    </div>
  )
}
