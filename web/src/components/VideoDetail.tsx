import { Fragment, useLayoutEffect, useRef, useState, type JSX } from 'react'
import { Link } from 'react-router-dom'
import { ChevronDown } from 'lucide-react'
import { avatarMediaUrl } from '@/api/client'
import type { ChannelListItem, Video } from '@/api/types'
import { formatDate, formatDateTime, formatRelativeTime } from '@/lib/formatDateTime'
import { Thumbnail } from './Thumbnail'
import { PlaybackSpeedMenu } from './PlaybackSpeedMenu'
import { VideoActionsMenu } from './VideoActionsMenu'
import { Badge } from '@/components/ui/badge'
import { cn } from '@/lib/utils'

const STATUS_LABELS: Record<string, string> = {
  IN_PROGRESS: 'Downloading',
  PENDING: 'Pending',
  ERRORED_RETRYING: 'Retrying',
  ERRORED: 'Errored',
}

// Tailwind's `md` breakpoint: details start expanded from here up.
const DESKTOP_QUERY = '(min-width: 48rem)'

// A web URL, not counting trailing punctuation such as a sentence's full stop.
const URL_PATTERN = /(https?:\/\/[^\s]*[^\s.,;:!?)\]'"])/g

interface VideoDetailProps {
  basePath: string
  video: Video
  channel?: ChannelListItem
  playbackRate: number
  onPlaybackRateChange: (rate: number) => void
}

/**
 * The selected video's detail pane in a playlist or channel detail view.
 * `channel` is only given in a channel view, where its avatar is shown and
 * links to the channel; in a playlist view the meta line names the video's
 * channel instead. `playbackRate` is the player's current speed, shown in
 * the title row's speed control.
 */
export function VideoDetail({
  basePath,
  video,
  channel,
  playbackRate,
  onPlaybackRateChange,
}: VideoDetailProps) {
  const [expanded, setExpanded] = useState(() => window.matchMedia(DESKTOP_QUERY).matches)
  const path = video.filename ? `${basePath}/${video.filename}` : basePath

  return (
    <div className="rounded-lg border border-border p-4">
      <div className="flex items-start gap-2.5">
        {channel && (
          <Link to={`/channels/${channel.id}`} className="shrink-0" aria-label={channel.name}>
            <Thumbnail
              src={channel.avatar_filename ? avatarMediaUrl(channel.avatar_filename) : null}
              className="mt-0.5 block size-8 rounded-full object-cover"
            />
          </Link>
        )}
        <h3 className="min-w-0 flex-1 font-heading text-lg font-semibold text-foreground md:text-xl">
          {video.title}
        </h3>
        <PlaybackSpeedMenu
          rate={playbackRate}
          onRateChange={onPlaybackRateChange}
          disabled={video.status !== 'DOWNLOADED'}
          className="mt-0.5"
        />
        <VideoActionsMenu
          videoId={video.id}
          title={video.title}
          markable={!video.watched && video.status === 'DOWNLOADED'}
          className="mt-0.5"
        />
      </div>
      <MetaLine video={video} showChannelName={!channel} />
      {expanded && (
        <div className="mt-3 mb-1">
          {video.status !== 'DOWNLOADED' && (
            <Badge variant="outline" className="mb-2">
              {STATUS_LABELS[video.status] ?? video.status}
            </Badge>
          )}
          {video.description && <Description key={video.id} text={video.description} />}
          <p className="mt-2 text-xs break-words text-muted-foreground">{path}</p>
          <a
            className="mt-3 inline-block text-sm text-primary underline-offset-4 hover:underline"
            href={`https://www.youtube.com/watch?v=${encodeURIComponent(video.id)}`}
            target="_blank"
            rel="noopener"
          >
            Open on YouTube
          </a>
        </div>
      )}
      <button
        type="button"
        className="mt-2 flex items-center gap-1 text-sm text-muted-foreground transition-colors hover:text-foreground"
        onClick={() => setExpanded((value) => !value)}
        aria-expanded={expanded}
      >
        {expanded ? 'Less' : 'More'}
        <ChevronDown className={cn('size-4 transition-transform', expanded && 'rotate-180')} />
      </button>
    </div>
  )
}

/**
 * `[<channel> · ]Published <date> · Synced <relative>`, omitting any part
 * whose value is absent, and nothing at all when every part is.
 */
function MetaLine({ video, showChannelName }: { video: Video; showChannelName: boolean }) {
  const parts = [
    showChannelName && video.channel_name && <span key="channel">{video.channel_name}</span>,
    video.published_at && <span key="published">Published {formatDate(video.published_at)}</span>,
    video.synced_at && (
      <span key="synced" title={formatDateTime(video.synced_at)}>
        Synced {formatRelativeTime(video.synced_at)}
      </span>
    ),
  ].filter((part): part is JSX.Element => Boolean(part))

  if (parts.length === 0) {
    return null
  }

  return (
    <p className="mt-1 text-sm text-muted-foreground">
      {parts.map((part, index) => (
        <Fragment key={part.key}>
          {index > 0 && ' · '}
          {part}
        </Fragment>
      ))}
    </p>
  )
}

/**
 * The description clamped to 4 lines, with a "Show more" toggle shown only
 * when the text overflows the clamp. Line breaks are kept and URLs are
 * rendered as links opening in a new tab.
 */
function Description({ text }: { text: string }) {
  const ref = useRef<HTMLParagraphElement>(null)
  const [showAll, setShowAll] = useState(false)
  const [overflows, setOverflows] = useState(false)

  useLayoutEffect(() => {
    const element = ref.current
    if (!element || showAll) {
      return
    }
    const measure = () => setOverflows(element.scrollHeight > element.clientHeight)
    measure()
    const observer = new ResizeObserver(measure)
    observer.observe(element)
    return () => observer.disconnect()
  }, [text, showAll])

  return (
    <div className="mb-2">
      <p
        ref={ref}
        className={cn(
          'text-sm break-words whitespace-pre-line text-foreground',
          !showAll && 'line-clamp-4',
        )}
      >
        {text.split(URL_PATTERN).map((segment, index) =>
          index % 2 === 1 ? (
            <a
              key={index}
              href={segment}
              target="_blank"
              rel="noopener"
              className="text-primary underline-offset-4 hover:underline"
            >
              {segment}
            </a>
          ) : (
            segment
          ),
        )}
      </p>
      {(overflows || showAll) && (
        <button
          type="button"
          className="mt-1 text-sm text-muted-foreground transition-colors hover:text-foreground"
          onClick={() => setShowAll((value) => !value)}
        >
          {showAll ? 'Show less' : 'Show more'}
        </button>
      )}
    </div>
  )
}
