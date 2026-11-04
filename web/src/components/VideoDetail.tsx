import { Fragment, type JSX } from 'react'
import { Link } from 'react-router-dom'
import { ExternalLink } from 'lucide-react'
import { avatarMediaUrl } from '@/api/client'
import type { ChannelListItem, Video } from '@/api/types'
import { formatDate, formatDateTime, formatRelativeTime } from '@/lib/formatDateTime'
import { Thumbnail } from './Thumbnail'
import { PlaybackSpeedMenu } from './PlaybackSpeedMenu'
import { MarkWatchedButton } from './MarkWatchedButton'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { Badge } from '@/components/ui/badge'
import { storageLocation } from '@/lib/storageLocation'
import { channelPath } from '@/lib/sourcePath'

const STATUS_LABELS: Record<string, string> = {
  IN_PROGRESS: 'Downloading',
  PENDING: 'Pending',
  ERRORED_RETRYING: 'Retrying',
  ERRORED: 'Errored',
}

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
 * channel instead. The title row ends with the video's own actions: "Mark
 * watched" (until it is watched), the speed control for the player's
 * current `playbackRate`, and a link to the video on YouTube.
 */
export function VideoDetail({
  basePath,
  video,
  channel,
  playbackRate,
  onPlaybackRateChange,
}: VideoDetailProps) {
  const location = storageLocation(basePath, video.filename)

  return (
    <div className="rounded-lg border border-border p-4">
      <div className="flex items-start gap-2.5">
        {channel && (
          <Link to={channelPath(channel.id)} className="shrink-0" aria-label={channel.name}>
            <Thumbnail
              src={channel.avatar_filename ? avatarMediaUrl(channel.avatar_filename) : null}
              className="mt-0.5 block size-8 rounded-full object-cover"
            />
          </Link>
        )}
        <h3 className="min-w-0 flex-1 font-heading text-lg font-semibold text-foreground md:text-xl">
          {video.title}
        </h3>
        <div className="mt-0.5 flex shrink-0 items-center gap-1">
          <MarkWatchedButton
            videoId={video.id}
            title={video.title}
            watched={video.watched}
            downloaded={video.status === 'DOWNLOADED'}
            className="mr-1"
          />
          <PlaybackSpeedMenu
            rate={playbackRate}
            onRateChange={onPlaybackRateChange}
            disabled={video.status !== 'DOWNLOADED'}
          />
          <YouTubeLink videoId={video.id} />
        </div>
      </div>
      <MetaLine video={video} showChannelName={!channel} />
      {video.summary && <AiSummary summary={video.summary} />}
      <div className="mt-2">
        {video.status !== 'DOWNLOADED' && (
          <Badge variant="outline" className="mb-2">
            {STATUS_LABELS[video.status] ?? video.status}
          </Badge>
        )}
        <p className="text-xs break-words text-muted-foreground">{location}</p>
      </div>
    </div>
  )
}

/** The video's AI summary, in full; its line breaks are kept. */
function AiSummary({ summary }: { summary: string }) {
  const label = 'AI summary'
  return (
    <section aria-label={label} className="mt-3">
      <h4 className="text-xs font-medium text-muted-foreground">{label}</h4>
      <p className="mt-1 text-sm break-words whitespace-pre-line text-foreground">{summary}</p>
    </section>
  )
}

/** An icon link opening the video on YouTube in a new tab. */
function YouTubeLink({ videoId }: { videoId: string }) {
  const label = 'Open video on YouTube'
  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <a
            href={`https://www.youtube.com/watch?v=${encodeURIComponent(videoId)}`}
            target="_blank"
            rel="noopener"
            aria-label={label}
            className="flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground"
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
 * `[Watched · ][<channel> · ]Published <date> · Synced <relative>`, omitting
 * any part whose value is absent, and nothing at all when every part is.
 * "Synced" shows only from the desktop breakpoint, so the line fits a phone.
 */
function MetaLine({ video, showChannelName }: { video: Video; showChannelName: boolean }) {
  const parts = [
    video.watched && <span key="watched">Watched</span>,
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
      {parts.map((part, index) =>
        part.key === 'synced' ? (
          // Hidden with its separator below the desktop breakpoint.
          <span key={part.key} className="max-md:hidden">
            {index > 0 && ' · '}
            {part}
          </span>
        ) : (
          <Fragment key={part.key}>
            {index > 0 && ' · '}
            {part}
          </Fragment>
        ),
      )}
    </p>
  )
}
