import { avatarMediaUrl } from '@/api/client'
import type { CategoryVideo } from '@/api/types'
import type { VideoSelection } from '@/hooks/useVideoSelection'
import { channelPath, playlistPath } from '@/lib/sourcePath'
import { VIDEO_SORT_LABELS, type VideoSort } from '@/lib/sourceVideos'
import { WatchView } from './WatchView'

interface CategoryPlayerProps {
  videos: CategoryVideo[] | undefined
  error: Error | null
  selection: VideoSelection<CategoryVideo>
  /** The order `videos` is in, named in the queue header. */
  sort: VideoSort
}

/**
 * A category played like a playlist: the player and the selected video's
 * info, linking its channel or playlist, with the category's videos, in
 * `sort` order, as the queue beside them.
 */
export function CategoryPlayer({ videos, error, selection, sort }: CategoryPlayerProps) {
  return (
    <WatchView
      noun="category"
      videos={videos}
      error={error}
      selection={selection}
      basePathOf={(item) => item.source.path}
      sourceNameOf={(item) => item.source.name}
      detailSourceOf={({ source }) =>
        source.kind === 'channel'
          ? {
              kind: 'channel',
              name: source.name,
              href: channelPath(source.id),
              avatarSrc: source.avatar_filename ? avatarMediaUrl(source.avatar_filename) : null,
            }
          : { kind: 'playlist', name: source.name, href: playlistPath(source.id) }
      }
      orderLabel={VIDEO_SORT_LABELS[sort].toLowerCase()}
    />
  )
}
