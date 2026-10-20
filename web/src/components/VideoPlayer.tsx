import { type VideoSourceRef, videoMediaUrl, videoSubtitlesUrl } from '@/api/client'
import type { Video } from '@/api/types'
import { languageName } from '@/lib/languageName'
import { cn } from '@/lib/utils'

interface VideoPlayerProps {
  basePath: string
  source: VideoSourceRef
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
export function VideoPlayer({
  basePath,
  source,
  video,
  autoplay,
  onVideoElement,
}: VideoPlayerProps) {
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
        <video
          ref={onVideoElement}
          controls
          autoPlay={autoplay}
          className="block h-full w-full md:h-auto md:max-h-[70vh] md:rounded-lg"
          src={videoMediaUrl(basePath, filename)}
          poster={
            video.thumbnail_filename ? videoMediaUrl(basePath, video.thumbnail_filename) : undefined
          }
        >
          {video.subtitle_language && (
            // Not `default`: captions stay off until turned on from the player's own controls.
            // Keyed per video so a new track starts off; the <video> itself isn't remounted.
            <track
              key={video.id}
              kind="subtitles"
              src={videoSubtitlesUrl(source, video.id)}
              srcLang={video.subtitle_language}
              label={languageName(video.subtitle_language)}
            />
          )}
        </video>
      ) : (
        <p className="text-sm text-muted-foreground">
          {video ? 'This video has not been downloaded yet.' : 'Select a video to play it.'}
        </p>
      )}
    </div>
  )
}
