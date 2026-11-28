import { useId } from 'react'
import { useSearchParams } from 'react-router-dom'
import { Star } from 'lucide-react'
import { useFavoriteVideos } from '@/api/queries'
import { useVideoSelection } from '@/hooks/useVideoSelection'
import { videoPathIn } from '@/lib/videoPath'
import { CategoryPlayer } from './CategoryPlayer'
import { FAVORITES_PATH } from './SidebarFavorites'
import { VideoGrid } from './VideoGrid'
import { VideoGridSkeleton } from './VideoSkeletons'

/** What the favorites view shows before anything is favorited. */
function NoFavorites() {
  const headingId = useId()
  return (
    <section
      aria-labelledby={headingId}
      className="flex flex-col items-center gap-4 rounded-xl border border-dashed border-border px-6 py-16 text-center"
    >
      <span className="flex size-12 items-center justify-center rounded-full bg-secondary text-muted-foreground">
        <Star className="size-6" />
      </span>
      <div className="flex max-w-sm flex-col gap-1.5">
        <h2 id={headingId} className="font-heading text-lg font-semibold">
          No favorites yet
        </h2>
        <p className="text-sm text-muted-foreground">
          Press the star on a video to add it to your favorites.
        </p>
      </div>
    </section>
  )
}

/**
 * The Favorites view (`/favorites`): every favorite video as a card naming
 * its source, in the server's order (most recently favorited first); or,
 * once one is picked (`?video=`), playing it with the favorites as the queue
 * beside the player, as on a category's view.
 */
export function FavoritesDetail() {
  const [searchParams] = useSearchParams()
  const playing = searchParams.has('video')
  const { data: videos, error, isFetchedAfterMount: videosFresh } = useFavoriteVideos()
  const selection = useVideoSelection(videos)

  return (
    <div
      className={
        playing ? 'flex flex-col gap-4 md:h-full md:min-h-[480px]' : 'flex min-w-0 flex-col gap-4'
      }
    >
      <h1 className="font-heading text-2xl font-bold text-foreground">Favorites</h1>
      {playing ? (
        <CategoryPlayer
          noun="list"
          videos={videos}
          videosFresh={videosFresh}
          error={error}
          selection={selection}
          orderLabel="most recently favorited first"
        />
      ) : !videos ? (
        // A failed background refresh keeps the loaded videos on screen.
        error ? (
          <p className="text-sm text-destructive">Failed to load favorites: {error.message}</p>
        ) : (
          <VideoGridSkeleton count={12} />
        )
      ) : videos.length === 0 ? (
        <NoFavorites />
      ) : (
        <VideoGrid
          videos={videos}
          layout="feed"
          videoPathOf={(video) => videoPathIn(FAVORITES_PATH, video.id)}
        />
      )}
    </div>
  )
}
