import { useState } from 'react'
import { useParams } from 'react-router-dom'
import { CheckCheck } from 'lucide-react'
import {
  useCategories,
  useCategoryVideos,
  useChannels,
  useMarkCategoryWatched,
  usePlaylists,
} from '@/api/queries'
import type { Category, ChannelListItem, PlaylistListItem } from '@/api/types'
import { byName } from '@/lib/categories'
import { errorMessage } from '@/lib/errorMessage'
import { Button } from '@/components/ui/button'
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

  if (!category) {
    return null
  }

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
          <p className="text-sm text-muted-foreground">
            {memberNames(category, channels, playlists).join(' · ')}
          </p>
        </div>
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
      </header>
      {videos && <VideoGrid videos={videos} />}
    </div>
  )
}
