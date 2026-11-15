import { Fragment, type JSX } from 'react'
import { Link } from 'react-router-dom'
import { ExternalLink } from 'lucide-react'
import type { Video } from '@/api/types'
import { formatDate } from '@/lib/formatDateTime'
import { Thumbnail } from './Thumbnail'
import { PlaylistIcon } from './PlaylistIcon'
import { PlaybackSpeedMenu } from './PlaybackSpeedMenu'
import { MarkWatchedButton } from './MarkWatchedButton'
import { ChapterList } from './ChapterList'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { Badge } from '@/components/ui/badge'
import { useIsDesktop } from '@/hooks/useIsDesktop'
import { storageLocation } from '@/lib/storageLocation'
import { canRetry, failingStepLines, statusLabel } from '@/lib/videoSteps'
import { RetryVideoButton } from './RetryVideoButton'

/** Where the video comes from, as its meta line links it. */
export interface VideoDetailSource {
  kind: 'channel' | 'playlist'
  name: string
  href: string
  avatarSrc?: string | null
}

interface VideoDetailProps {
  basePath: string
  video: Video
  /**
   * The channel or playlist to link on the meta line; absent (in a
   * playlist's own view), the video's channel is named instead.
   */
  source?: VideoDetailSource
  /** E.g. "Episode 4 of 32", in a playlist. */
  episodeLabel?: string | null
  playbackRate: number
  onPlaybackRateChange: (rate: number) => void
  /** The playing `<video>`, which the chapter list seeks (desktop). */
  videoElement?: HTMLVideoElement | null
}

/**
 * The selected video's info under the player: its title; a meta line with
 * its source (avatar and link), publish date and episode; its own actions
 * grouped right ("Mark watched" until watched, the speed control, the
 * YouTube link); then its AI summary with, on desktop, its chapters listed
 * beside it.
 */
export function VideoDetail({
  basePath,
  video,
  source,
  episodeLabel,
  playbackRate,
  onPlaybackRateChange,
  videoElement = null,
}: VideoDetailProps) {
  const isDesktop = useIsDesktop()
  const location = storageLocation(basePath, video.filename)
  const showChapters = isDesktop && video.status === 'DOWNLOADED' && video.chapters.length > 0

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-start gap-3">
        <div className="flex min-w-0 flex-[1_1_20rem] flex-col gap-2">
          <h3 className="font-heading text-lg leading-tight font-bold text-foreground md:text-[1.375rem]">
            {video.title}
          </h3>
          <MetaLine video={video} source={source} episodeLabel={episodeLabel} />
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <MarkWatchedButton
            videoId={video.id}
            title={video.title}
            watched={video.watched}
            downloaded={video.status === 'DOWNLOADED'}
          />
          <PlaybackSpeedMenu
            rate={playbackRate}
            onRateChange={onPlaybackRateChange}
            disabled={video.status !== 'DOWNLOADED'}
            className="h-9 rounded-full bg-secondary px-3 text-foreground"
          />
          <YouTubeLink youtubeId={video.youtube_id} />
        </div>
      </div>
      {(video.summary || showChapters) && (
        <div className="flex flex-wrap gap-4">
          {video.summary && <AiSummary summary={video.summary} />}
          {showChapters && <ChapterList chapters={video.chapters} videoElement={videoElement} />}
        </div>
      )}
      <div>
        {video.status !== 'DOWNLOADED' && (
          <Badge variant="outline" className="mb-2">
            {statusLabel(video.status)}
          </Badge>
        )}
        <FailingSteps video={video} />
        {canRetry(video.steps) && <RetryVideoButton videoId={video.id} title={video.title} />}
        <p className="text-xs break-words text-muted-foreground">{location}</p>
      </div>
    </div>
  )
}

/** One line per step of the video that is retrying or failed. */
function FailingSteps({ video }: { video: Video }) {
  const lines = failingStepLines(video.steps, new Date())
  if (lines.length === 0) {
    return null
  }
  return (
    <ul aria-label="Failing steps" className="mb-2 text-xs text-muted-foreground">
      {lines.map((line) => (
        <li key={line}>{line}</li>
      ))}
    </ul>
  )
}

/** The video's AI summary, in full, as a panel; its line breaks are kept. */
function AiSummary({ summary }: { summary: string }) {
  const label = 'AI summary'
  return (
    <section
      aria-label={label}
      className="flex min-w-0 flex-[1_1_18rem] flex-col gap-2 rounded-xl bg-secondary/70 p-4 dark:bg-card"
    >
      <h4 className="text-xs font-semibold tracking-wider text-muted-foreground uppercase">
        {label}
      </h4>
      <p className="text-sm leading-relaxed break-words whitespace-pre-line text-foreground">
        {summary}
      </p>
    </section>
  )
}

/** An icon link opening the video on YouTube in a new tab. */
function YouTubeLink({ youtubeId }: { youtubeId: string }) {
  const label = 'Open video on YouTube'
  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <a
            href={`https://www.youtube.com/watch?v=${encodeURIComponent(youtubeId)}`}
            target="_blank"
            rel="noopener"
            aria-label={label}
            className="flex size-9 shrink-0 items-center justify-center rounded-full bg-secondary text-foreground transition-colors hover:bg-accent"
          >
            <ExternalLink className="size-4" />
          </a>
        </TooltipTrigger>
        <TooltipContent>{label}</TooltipContent>
      </Tooltip>
    </TooltipProvider>
  )
}

/**
 * `[<source>|<channel> · ][Watched · ]Published <date>[ · <episode>]`,
 * omitting any part whose value is absent, and nothing at all when every
 * part is. The source is its avatar (or playlist icon) and name, linked.
 */
function MetaLine({
  video,
  source,
  episodeLabel,
}: {
  video: Video
  source: VideoDetailSource | undefined
  episodeLabel: string | null | undefined
}) {
  const parts = [
    source ? (
      <Link
        key="source"
        to={source.href}
        className="inline-flex min-w-0 items-center gap-1.5 font-semibold text-foreground no-underline hover:text-primary"
      >
        {source.kind === 'channel' ? (
          <Thumbnail
            src={source.avatarSrc ?? null}
            className="block size-5.5 shrink-0 rounded-full object-cover"
          />
        ) : (
          <PlaylistIcon className="size-5.5" />
        )}
        <span className="truncate">{source.name}</span>
      </Link>
    ) : (
      video.channel_name && <span key="channel">{video.channel_name}</span>
    ),
    video.watched && <span key="watched">Watched</span>,
    video.published_at && <span key="published">Published {formatDate(video.published_at)}</span>,
    episodeLabel && <span key="episode">{episodeLabel}</span>,
  ].filter((part): part is JSX.Element => Boolean(part))

  if (parts.length === 0) {
    return null
  }

  return (
    <p className="flex min-w-0 flex-wrap items-center gap-x-1.5 text-[0.8125rem] text-muted-foreground">
      {parts.map((part, index) => (
        <Fragment key={part.key}>
          {index > 0 && <span aria-hidden="true">·</span>}
          {part}
        </Fragment>
      ))}
    </p>
  )
}
