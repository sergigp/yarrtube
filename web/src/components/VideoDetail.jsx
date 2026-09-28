import { useState } from 'react'
import { Link } from 'react-router-dom'
import { ChevronDown } from 'lucide-react'
import { avatarMediaUrl } from '../api'
import { Thumbnail } from './Thumbnail'
import { Badge } from '@/components/ui/badge'
import { cn } from '@/lib/utils'

const STATUS_LABELS = {
  DOWNLOADED: 'Downloaded',
  IN_PROGRESS: 'Downloading',
  PENDING: 'Pending',
  ERRORED_RETRYING: 'Retrying',
  ERRORED: 'Errored',
}

const QUALITY_LABELS = {
  high: 'High quality',
  mid: 'Medium quality',
  low: 'Low quality',
}

// Tailwind's `md` breakpoint: details start expanded from here up.
const DESKTOP_QUERY = '(min-width: 48rem)'

/**
 * The selected video's detail pane in a playlist or channel detail view.
 * `channel` is only given in a channel view, where its avatar is shown and
 * links to the channel.
 */
export function VideoDetail({ basePath, video, channel }) {
  const [expanded, setExpanded] = useState(() => window.matchMedia(DESKTOP_QUERY).matches)
  const path = video.filename ? `${basePath}/${video.filename}` : basePath

  return (
    <div className="rounded-lg border border-border p-4">
      <div className="flex items-start gap-2.5">
        {channel && (
          <Link
            to={`/channels/${channel.id}`}
            className="shrink-0"
            aria-label={channel.name}
          >
            <Thumbnail
              src={channel.avatar_filename ? avatarMediaUrl(channel.avatar_filename) : null}
              className="mt-0.5 block size-8 rounded-full object-cover"
            />
          </Link>
        )}
        <h3 className="min-w-0 flex-1 font-heading text-lg font-semibold text-foreground md:text-xl">
          {video.title}
        </h3>
      </div>
      {expanded && (
        <div className="mt-3 mb-1">
          <div className="flex flex-wrap items-center gap-1.5">
            <Badge variant={video.status === 'DOWNLOADED' ? 'secondary' : 'outline'}>
              {STATUS_LABELS[video.status] ?? video.status}
            </Badge>
            <Badge variant="outline">{QUALITY_LABELS[video.quality] ?? '—'}</Badge>
          </div>
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
