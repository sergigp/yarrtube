import { avatarMediaUrl } from '../api'
import { Thumbnail } from './Thumbnail'
import { Badge } from '@/components/ui/badge'

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

/**
 * The selected video's detail pane in a playlist or channel detail view.
 * `channel` is only given in a channel view, where its avatar is shown.
 */
export function VideoDetail({ basePath, video, channel }) {
  const path = video.filename ? `${basePath}/${video.filename}` : basePath

  return (
    <div className="rounded-lg border border-border p-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="flex min-w-0 flex-1 items-start gap-2.5">
          {channel && (
            <Thumbnail
              src={channel.avatar_filename ? avatarMediaUrl(channel.avatar_filename) : null}
              className="mt-0.5 size-8 shrink-0 rounded-full object-cover"
            />
          )}
          <h3 className="min-w-0 flex-1 font-heading text-xl font-semibold text-foreground">
            {video.title}
          </h3>
        </div>
        <div className="flex shrink-0 items-center gap-1.5">
          <Badge variant={video.status === 'DOWNLOADED' ? 'secondary' : 'outline'}>
            {STATUS_LABELS[video.status] ?? video.status}
          </Badge>
          <Badge variant="outline">{QUALITY_LABELS[video.quality] ?? '—'}</Badge>
        </div>
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
  )
}
