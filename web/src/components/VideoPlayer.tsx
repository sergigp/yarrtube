import { useCallback, useState } from 'react'
import { ChevronLeft, ChevronRight } from 'lucide-react'
import { videoMediaUrl, videoSubtitlesUrl } from '@/api/client'
import type { Chapter, Video } from '@/api/types'
import { useCurrentTime } from '@/hooks/useCurrentTime'
import { type SeekFeedback, useSeekKeys } from '@/hooks/useSeekKeys'
import { useAutoplay } from '@/hooks/useAutoplay'
import { useIsDesktop } from '@/hooks/useIsDesktop'
import { useLandscapeTakeover } from '@/hooks/useLandscapeTakeover'
import { languageName } from '@/lib/languageName'
import { playFrom } from '@/lib/playFrom'
import { cn } from '@/lib/utils'
import { ChapterRail } from './ChapterRail'

interface VideoPlayerProps {
  basePath: string
  video: Video | null
  autoplay: boolean
  /**
   * Whether `video` comes from a list fetched since the page opened, not
   * only from the cache, which can still show it downloading.
   */
  videoFresh: boolean
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
 * (cancelling the page gutter), sticks right under the header while the
 * page scrolls, and carries the chapter rail; on desktop the chapters are
 * listed beside the video's AI summary instead (`ChapterList`). On a phone
 * held in landscape a downloaded video takes over the whole screen.
 */
export function VideoPlayer({
  basePath,
  video,
  autoplay,
  videoFresh,
  onVideoElement,
}: VideoPlayerProps) {
  // The downloaded file to play, or null while the video can't play yet.
  const filename = video?.status === 'DOWNLOADED' ? video.filename : null
  const [videoElement, setVideoElement] = useState<HTMLVideoElement | null>(null)
  const seekFeedback = useSeekKeys(videoElement)
  const currentTime = useCurrentTime(videoElement)
  const pickChapter = useCallback(
    (chapter: Chapter) => {
      if (videoElement) {
        playFrom(videoElement, chapter.start_seconds)
      }
    },
    [videoElement],
  )
  const videoRef = useCallback(
    (element: HTMLVideoElement | null) => {
      setVideoElement(element)
      onVideoElement(element)
    },
    [onVideoElement],
  )

  useAutoplay(videoElement, video?.id ?? null, filename !== null, videoFresh, autoplay)
  const takeover = useLandscapeTakeover(filename !== null)

  const isDesktop = useIsDesktop()
  const chapters = video && filename && !isDesktop && !takeover ? video.chapters : []

  return (
    <div
      // Pinned below the header on phones, chapter rail included, which keeps the header in view.
      data-pinned-player=""
      data-landscape-takeover={takeover ? '' : undefined}
      className={
        takeover
          ? // A phone in landscape: only the video, on black, clear of the notch. Same
            // elements as in portrait, so the <video> isn't remounted and keeps playing.
            'fixed inset-0 z-30 flex bg-black pt-[env(safe-area-inset-top)] pr-[env(safe-area-inset-right)] pb-[env(safe-area-inset-bottom)] pl-[env(safe-area-inset-left)]'
          : cn(
              'sticky top-(--header-offset) z-10 -mx-4 flex flex-col bg-background sm:-mx-6',
              'md:relative md:top-auto md:z-auto md:m-0 md:shrink-0 md:gap-2',
            )
      }
    >
      <div
        className={
          takeover
            ? 'relative flex min-h-0 flex-1 items-center justify-center overflow-hidden'
            : cn(
                'relative flex overflow-hidden aspect-video items-center justify-center',
                'md:aspect-auto md:min-h-80 md:rounded-xl',
                filename ? 'bg-black md:bg-secondary/60' : 'bg-secondary md:bg-secondary/60',
              )
        }
      >
        {video && filename ? (
          <video
            ref={videoRef}
            controls
            // iOS Safari goes fullscreen on play without it; stay inline so the chapter
            // rail stays reachable, fullscreen remains one tap away in the controls.
            playsInline
            className={
              takeover
                ? 'block h-full w-full object-contain'
                : 'block h-full w-full md:h-auto md:max-h-[70vh] md:rounded-xl'
            }
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
                src={videoSubtitlesUrl(video.id)}
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
      {chapters.length > 0 && (
        <ChapterRail chapters={chapters} currentTime={currentTime} onPick={pickChapter} />
      )}
    </div>
  )
}
