import { useParams } from 'react-router-dom'
import { useCategories, useCategoryVideos, useChannels, usePlaylists } from '@/api/queries'
import type { Category, ChannelListItem, PlaylistListItem } from '@/api/types'
import { byName } from '@/lib/categories'
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
  const { data } = useCategories()
  const { data: channels } = useChannels()
  const { data: playlists } = usePlaylists()
  const { data: videos } = useCategoryVideos(id)
  const category = data?.categories.find((candidate) => candidate.id === id)

  if (!category) {
    return null
  }

  return (
    <div className="flex min-w-0 flex-col gap-6">
      <header className="flex min-w-0 flex-col gap-1">
        <h1 className="font-heading text-xl font-semibold">{category.name}</h1>
        <p className="text-sm text-muted-foreground">
          {memberNames(category, channels, playlists).join(' · ')}
        </p>
      </header>
      {videos && <VideoGrid videos={videos} />}
    </div>
  )
}
