import { useCallback, useState } from 'react'
import { ChevronLeft, ChevronRight } from 'lucide-react'
import { type VideoSourceRef, videoMediaUrl, videoSubtitlesUrl } from '@/api/client'
import type { Chapter, Video } from '@/api/types'
import { useCurrentTime } from '@/hooks/useCurrentTime'
import { type SeekFeedback, useSeekKeys } from '@/hooks/useSeekKeys'
import { languageName } from '@/lib/languageName'
import { cn } from '@/lib/utils'
import { ChapterRail } from './ChapterRail'

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

/** Moves `element` to `seconds` and plays it. */
function playFrom(element: HTMLVideoElement, seconds: number) {
  element.currentTime = seconds
  // A rejected play (e.g. autoplay policy) leaves it paused there.
  element.play().catch(() => {})
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

  const chapters = video && filename ? video.chapters : []

  return (
    <div
      // Pinned below the header on phones, chapter rail included, which keeps the header in view.
      data-pinned-player=""
      className={cn(
        'sticky top-(--header-offset) z-10 -mx-4 flex flex-col bg-background sm:-mx-6',
        'md:relative md:top-auto md:z-auto md:m-0 md:shrink-0 md:gap-2',
      )}
    >
      <div
        className={cn(
          'relative flex overflow-hidden aspect-video items-center justify-center',
          'md:aspect-auto md:min-h-80 md:rounded-lg',
          filename ? 'bg-black md:bg-secondary/60' : 'bg-secondary md:bg-secondary/60',
        )}
      >
        {video && filename ? (
          <video
            ref={videoRef}
            controls
            // iOS Safari goes fullscreen on play without it; stay inline so the chapter
            // rail stays reachable, fullscreen remains one tap away in the controls.
            playsInline
            autoPlay={autoplay}
            className={cn(
            'block h-full w-full md:h-auto md:rounded-lg',
            // With chapters, the rail (4.5 rem cards + 0.5 rem gap) comes out of the same 70vh.
            chapters.length > 0 ? 'md:max-h-[calc(70vh-5rem)]' : 'md:max-h-[70vh]',
          )}
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
      {chapters.length > 0 && (
        <ChapterRail chapters={chapters} currentTime={currentTime} onPick={pickChapter} />
      )}
    </div>
  )
}
