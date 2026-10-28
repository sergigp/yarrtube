import { useCallback, useState } from 'react'
import { ChevronLeft, ChevronRight } from 'lucide-react'
import { type VideoSourceRef, videoMediaUrl, videoSubtitlesUrl } from '@/api/client'
import type { Video } from '@/api/types'
import { type SeekFeedback, useSeekKeys } from '@/hooks/useSeekKeys'
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
 * YouTube-style ripple over the side of the player being sought towards:
 * three chevrons and the seconds sought, fading out on its own. The caller keys
 * it per press so every press replays it.
 */
function SeekIndicator({ feedback }: { feedback: SeekFeedback }) {
  const back = feedback.direction === 'back'
  const Chevron = back ? ChevronLeft : ChevronRight
  return (
    <div
      className={cn(
        'pointer-events-none absolute inset-y-0 flex w-2/5 animate-seek-feedback flex-col items-center justify-center gap-1 text-white',
        back
          ? 'left-0 rounded-r-[50%] bg-linear-to-r from-black/40 to-black/20'
          : 'right-0 rounded-l-[50%] bg-linear-to-l from-black/40 to-black/20',
      )}
    >
      <div className={cn('flex', back && 'flex-row-reverse')} aria-hidden="true">
        {[0, 1, 2].map((index) => (
          <Chevron
            key={index}
            className="-mx-1.5 size-6 animate-seek-chevron"
            style={{ animationDelay: `${index * 0.1}s` }}
          />
        ))}
      </div>
      <span className="text-sm font-medium" role="status">
        {feedback.seconds} seconds
      </span>
    </div>
  )
}

/**
 * The player area of a playlist or channel detail view: plays `video` from
 * `basePath` once downloaded, or explains why it can't yet. ← / → seek 10 s
 * with on-screen feedback.
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
  const [videoElement, setVideoElement] = useState<HTMLVideoElement | null>(null)
  const seekFeedback = useSeekKeys(videoElement)
  const videoRef = useCallback(
    (element: HTMLVideoElement | null) => {
      setVideoElement(element)
      onVideoElement(element)
    },
    [onVideoElement],
  )

  return (
    <div
      className={cn(
        'sticky top-(--header-height) z-10 -mx-4 flex overflow-hidden aspect-video items-center justify-center sm:-mx-6',
        'md:relative md:top-auto md:z-auto md:m-0 md:aspect-auto md:min-h-80 md:shrink-0 md:rounded-lg',
        filename ? 'bg-black md:bg-secondary/60' : 'bg-secondary md:bg-secondary/60',
      )}
    >
      {video && filename ? (
        <video
          ref={videoRef}
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
      {filename && seekFeedback && (
        <SeekIndicator key={seekFeedback.press} feedback={seekFeedback} />
      )}
    </div>
  )
}
