import { avatarMediaUrl } from '@/api/client'
import type { CategoryVideo } from '@/api/types'
import type { VideoSelection } from '@/hooks/useVideoSelection'
import { channelPath, playlistPath } from '@/lib/sourcePath'
import { WatchView } from './WatchView'

interface CategoryPlayerProps {
  videos: CategoryVideo[] | undefined
  /** Whether `videos` were fetched since the page opened, not only read from the cache. */
  videosFresh: boolean
  error: Error | null
  selection: VideoSelection<CategoryVideo>
  /** The order `videos` is in, named in the queue header, e.g. "newest first". */
  orderLabel: string
  /** Names what is played in the queue's empty message; "category" by default. */
  noun?: string
}

/**
 * A category (or the favorites) played like a playlist: the player and the
 * selected video's info, linking its channel or playlist, with the videos, in
 * `orderLabel` order, as the queue beside them.
 */
export function CategoryPlayer({
  videos,
  videosFresh,
  error,
  selection,
  orderLabel,
  noun = 'category',
}: CategoryPlayerProps) {
  return (
    <WatchView
      noun={noun}
      videos={videos}
      videosFresh={videosFresh}
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
      orderLabel={orderLabel}
    />
  )
}
