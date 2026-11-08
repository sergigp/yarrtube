import { useEffect, useId, useState } from 'react'
import { Link, useLocation, useNavigate, useParams, useSearchParams } from 'react-router-dom'
import { FolderPlus, Plus, Settings2 } from 'lucide-react'
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
import { CategoryPlayer } from './CategoryPlayer'
import { DetailHeader } from './DetailHeader'
import { CategoryDialog } from './CategoryDialog'
import { VideoGrid } from './VideoGrid'
import { VideoGridSkeleton } from './VideoSkeletons'

/** Router state that opens the category settings dialog on arrival at a category's view. */
export interface CategorySettingsState {
  editSettings: true
}

/**
 * What an empty category shows instead of videos: what it is for, and the
 * way to fill it, the category settings dialog.
 */
function EmptyCategory({ category, onAddSources }: { category: Category; onAddSources: () => void }) {
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
      <Button type="button" onClick={onAddSources}>
        <Plus />
        Add channels and playlists
      </Button>
    </section>
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
  const [editingSettings, setEditingSettings] = useState(false)
  // Arriving with CategorySettingsState opens the dialog once per navigation;
  // the state is then dropped from history so reload and back don't reopen it.
  const arrivedEditing = (location.state as CategorySettingsState | null)?.editSettings === true
  const [openedFor, setOpenedFor] = useState<string | null>(null)
  if (arrivedEditing && openedFor !== location.key) {
    setOpenedFor(location.key)
    setEditingSettings(true)
  }
  useEffect(() => {
    if (arrivedEditing) {
      navigate(location.pathname, { replace: true, state: null })
    }
  }, [arrivedEditing, location.pathname, navigate])
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
  // Videos set by hand or by the AI fill a category without sources too.
  const empty = sources === 0 && videos?.length === 0
  const hasVideos = sources > 0 || (videos?.length ?? 0) > 0

  return (
    <div className={playing ? 'flex flex-col md:h-full md:min-h-[480px]' : 'flex min-w-0 flex-col'}>
      <DetailHeader
        name={category.name}
        tint={softTint(category.color, theme)}
        videos={videos}
        unwatchedCount={category.unwatched_count}
        summaryPrefix={`${sources} ${sources === 1 ? 'source' : 'sources'}`}
        onRename={refreshing((name: string) => updateCategory(category.id, { name }))}
        extraActions={[
          { label: 'Edit settings', icon: <Settings2 />, onSelect: () => setEditingSettings(true) },
        ]}
        onMarkWatched={hasVideos ? () => markCategoryWatched(category.id) : undefined}
        onDelete={refreshing(async () => {
          await deleteCategory(category.id)
          navigate('/')
        })}
        deleteDescription={deleteCategoryDescription(category)}
      />
      {empty ? (
        <EmptyCategory category={category} onAddSources={() => setEditingSettings(true)} />
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
      <CategoryDialog category={category} open={editingSettings} onOpenChange={setEditingSettings} />
    </div>
  )
}
