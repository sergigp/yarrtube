import { useId, useState } from 'react'
import { useParams } from 'react-router-dom'
import { CheckCheck, FolderPlus, Plus, Settings2 } from 'lucide-react'
import {
  useCategories,
  useCategoryVideos,
  useChannels,
  useMarkCategoryWatched,
  usePlaylists,
} from '@/api/queries'
import type { Category, ChannelListItem, PlaylistListItem } from '@/api/types'
import { byName, memberCount } from '@/lib/categories'
import { errorMessage } from '@/lib/errorMessage'
import { Button } from '@/components/ui/button'
import { ManageSourcesDialog } from './ManageSourcesDialog'
import { VideoGrid } from './VideoGrid'

/** The names of the category's members: channels, then playlists, each by name. */
function memberNames(
  category: Category,
  channels: ChannelListItem[] | undefined,
  playlists: PlaylistListItem[] | undefined,
): string[] {
  return [
    ...byName(channels?.filter((channel) => category.channels.includes(channel.id))),
    ...byName(playlists?.filter((playlist) => category.playlists.includes(playlist.id))),
  ].map((source) => source.name)
}

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

/** A category's view (`/categories/:id`): its members and videos. */
export function CategoryDetail() {
  const { id = '' } = useParams()
  const markCategoryWatched = useMarkCategoryWatched()
  const [marking, setMarking] = useState(false)
  const [managing, setManaging] = useState(false)
  const { data } = useCategories()
  const { data: channels } = useChannels()
  const { data: playlists } = usePlaylists()
  const { data: videos } = useCategoryVideos(id)
  const category = data?.categories.find((candidate) => candidate.id === id)

  if (!data) {
    return <p className="text-sm text-muted-foreground">Loading category…</p>
  }
  if (!category) {
    return <p className="text-sm text-muted-foreground">Category not found.</p>
  }
  const empty = memberCount(category) === 0

  const markAllWatched = async () => {
    setMarking(true)
    try {
      await markCategoryWatched(category.id)
    } catch (err) {
      window.alert(`Failed to mark "${category.name}" watched: ${errorMessage(err)}`)
    } finally {
      setMarking(false)
    }
  }

  return (
    <div className="flex min-w-0 flex-col gap-6">
      <header className="flex min-w-0 flex-wrap items-start justify-between gap-4">
        <div className="flex min-w-0 flex-col gap-1">
          <h1 className="font-heading text-xl font-semibold">{category.name}</h1>
          {!empty && (
            <p className="text-sm text-muted-foreground">
              {memberNames(category, channels, playlists).join(' · ')}
            </p>
          )}
        </div>
        <div className="flex items-center gap-2">
          {!empty && (
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={marking}
              onClick={markAllWatched}
            >
              <CheckCheck />
              Mark all watched
            </Button>
          )}
          <Button type="button" variant="outline" size="sm" onClick={() => setManaging(true)}>
            <Settings2 />
            Manage
          </Button>
        </div>
      </header>
      {empty ? (
        <EmptyCategory category={category} onManage={() => setManaging(true)} />
      ) : (
        videos && <VideoGrid videos={videos} />
      )}
      <ManageSourcesDialog category={category} open={managing} onOpenChange={setManaging} />
    </div>
  )
}
