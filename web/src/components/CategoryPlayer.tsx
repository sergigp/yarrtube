import { useChannels } from '@/api/queries'
import { avatarMediaUrl } from '@/api/client'
import type { CategoryVideo } from '@/api/types'
import type { VideoSelection } from '@/hooks/useVideoSelection'
import { channelPath, playlistPath } from '@/lib/sourcePath'
import { WatchView } from './WatchView'

interface CategoryPlayerProps {
  videos: CategoryVideo[] | undefined
  error: Error | null
  selection: VideoSelection<CategoryVideo>
}

/**
 * A category played like a playlist: the player and the selected video's
 * info, linking its channel or playlist, with the category's videos as the
 * queue beside them.
 */
export function CategoryPlayer({ videos, error, selection }: CategoryPlayerProps) {
  const { data: channels } = useChannels()
  return (
    <WatchView
      noun="category"
      videos={videos}
      error={error}
      selection={selection}
      basePathOf={(item) => item.source.path}
      sourceNameOf={(item) => item.source.name}
      detailSourceOf={({ source }) => {
        const avatar =
          source.avatar_filename ??
          channels?.find((channel) => channel.id === source.id)?.avatar_filename ??
          null
        return source.kind === 'channel'
          ? {
              kind: 'channel',
              name: source.name,
              href: channelPath(source.id),
              avatarSrc: avatar ? avatarMediaUrl(avatar) : null,
            }
          : { kind: 'playlist', name: source.name, href: playlistPath(source.id) }
      }}
      orderLabel="newest first"
    />
  )
}
