import { useId, useState } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import { FolderPlus, Plus, Settings2 } from 'lucide-react'
import { deleteCategory, renameCategory } from '@/api/client'
import {
  useCategories,
  useCategoryVideos,
  useLibraryAction,
  useMarkCategoryWatched,
} from '@/api/queries'
import type { Category } from '@/api/types'
import { deleteCategoryDescription, memberCount } from '@/lib/categories'
import { Button } from '@/components/ui/button'
import { DropdownMenuItem } from '@/components/ui/dropdown-menu'
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

/** A category's view (`/categories/:id`): its header and videos. */
export function CategoryDetail() {
  const { id = '' } = useParams()
  const navigate = useNavigate()
  const refreshing = useLibraryAction()
  const markCategoryWatched = useMarkCategoryWatched()
  const [managing, setManaging] = useState(false)
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
        videos={videos}
        unwatchedCount={category.unwatched_count}
        summaryPrefix={`${sources} ${sources === 1 ? 'source' : 'sources'}`}
        onRename={refreshing((name: string) => renameCategory(category.id, name))}
        menuItems={
          <DropdownMenuItem onSelect={() => setManaging(true)}>
            <Settings2 />
            Manage sources
          </DropdownMenuItem>
        }
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
