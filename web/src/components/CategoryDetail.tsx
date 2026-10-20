import { useId, useState } from 'react'
import { Link, useParams } from 'react-router-dom'
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
import type { ManageCategoryState } from './AddCategoryDialog'
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

/**
 * What an empty category shows instead of videos: what it is for, and the
 * way to fill it, which lands on its add-members search in the management view.
 */
function EmptyCategory({ category, managePath }: { category: Category; managePath: string }) {
  const headingId = useId()
  const state: ManageCategoryState = { focusAddMembers: true }
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
      <Button asChild>
        <Link to={managePath} state={state}>
          <Plus />
          Add channels and playlists
        </Link>
      </Button>
    </section>
  )
}

/** A category's view (`/categories/:id`): its members and videos. */
export function CategoryDetail() {
  const { id = '' } = useParams()
  const markCategoryWatched = useMarkCategoryWatched()
  const [marking, setMarking] = useState(false)
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
  const managePath = `/categories?selected=${encodeURIComponent(category.id)}`
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
          <Button asChild variant="outline" size="sm">
            <Link to={managePath}>
              <Settings2 />
              Manage
            </Link>
          </Button>
        </div>
      </header>
      {empty ? (
        <EmptyCategory category={category} managePath={managePath} />
      ) : (
        videos && <VideoGrid videos={videos} />
      )}
    </div>
  )
}
