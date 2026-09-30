import { videoMediaUrl } from '@/api/client'
import type { Video } from '@/api/types'
import { cn } from '@/lib/utils'

interface VideoPlayerProps {
  basePath: string
  video: Video | null
  autoplay: boolean
  onVideoElement: (element: HTMLVideoElement | null) => void
}

/**
 * The player area of a playlist or channel detail view: plays `video` from
 * `basePath` once downloaded, or explains why it can't yet.
 *
 * Below the desktop breakpoint it bleeds to the side edges of the screen
 * (cancelling the page gutter) and sticks right under the header while the
 * page scrolls.
 */
export function VideoPlayer({ basePath, video, autoplay, onVideoElement }: VideoPlayerProps) {
  // The downloaded file to play, or null while the video can't play yet.
  const filename = video?.status === 'DOWNLOADED' ? video.filename : null

  return (
    <div
      className={cn(
        'sticky top-(--header-height) z-10 -mx-4 flex aspect-video items-center justify-center sm:-mx-6',
        'md:static md:m-0 md:aspect-auto md:min-h-80 md:shrink-0 md:rounded-lg',
        filename ? 'bg-black md:bg-secondary/60' : 'bg-secondary md:bg-secondary/60',
      )}
    >
      {video && filename ? (
        // eslint-disable-next-line jsx-a11y/media-has-caption
        <video
          ref={onVideoElement}
          controls
          autoPlay={autoplay}
          className="block h-full w-full md:h-auto md:max-h-[70vh] md:rounded-lg"
          src={videoMediaUrl(basePath, filename)}
          poster={
            video.thumbnail_filename ? videoMediaUrl(basePath, video.thumbnail_filename) : undefined
          }
        />
      ) : (
        <p className="text-sm text-muted-foreground">
          {video ? 'This video has not been downloaded yet.' : 'Select a video to play it.'}
        </p>
      )}
    </div>
  )
}
