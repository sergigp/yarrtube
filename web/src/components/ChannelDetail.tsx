import { useState } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import {
  queryKeys,
  useCategoriesShown,
  useCategorizationAvailable,
  useChannels,
  useChannelVideos,
  useDeleteSource,
  useLibraryAction,
} from '@/api/queries'
import {
  reconcileChannel,
  deleteChannel,
  deleteSourceAiVerdicts,
  markChannelWatched,
  avatarMediaUrl,
} from '@/api/client'
import { useSaveChannelSettings } from '@/hooks/useSaveChannelSettings'
import { useVideoSelection } from '@/hooks/useVideoSelection'
import { formatRelativeTime } from '@/lib/formatDateTime'
import { newestUnwatched, sourceSummary } from '@/lib/sourceVideos'
import { channelPath, youtubeChannelUrl } from '@/lib/sourcePath'
import { DetailHeader } from './DetailHeader'
import { EditChannelDialog } from './EditChannelDialog'
import { PlayLink } from './PlayLink'
import { WatchView } from './WatchView'

/**
 * A channel's page (`/channels/:id`), straight into the watching layout:
 * most channels keep only a few videos, so there's no grid to browse. Its
 * header's "Play next" plays the newest video not watched yet.
 */
export function ChannelDetail() {
  const { id = '' } = useParams()
  const navigate = useNavigate()
  const { data: channels, error: channelsError } = useChannels()
  const refreshing = useLibraryAction()
  const deleteSource = useDeleteSource()
  const saveChannelSettings = useSaveChannelSettings()
  const [editing, setEditing] = useState(false)
  const categoriesShown = useCategoriesShown()
  const categorizationAvailable = useCategorizationAvailable()
  const channel = channels?.find((item) => item.id === id) ?? null

  const { data: videos, error } = useChannelVideos(id)
  const selection = useVideoSelection(videos)

  // A failed background refresh keeps the loaded channel, and its player, on screen.
  if (channelsError && !channels) {
    return (
      <p className="text-sm text-destructive">Failed to load channel: {channelsError.message}</p>
    )
  }

  if (!channels) {
    return <p className="text-sm text-muted-foreground">Loading channel…</p>
  }

  if (!channel) {
    return <p className="text-sm text-destructive">Channel not found.</p>
  }

  const avatarSrc = channel.avatar_filename ? avatarMediaUrl(channel.avatar_filename) : null
  const next = videos ? newestUnwatched(videos) : null

  return (
    <div className="flex flex-col md:h-full md:min-h-[480px]">
      <DetailHeader
        name={channel.name}
        badge={{ kind: 'avatar', src: avatarSrc }}
        compact
        summary={
          videos
            ? sourceSummary({
                kind: 'Channel',
                videoCount: videos.length,
                unwatchedCount: channel.unwatched_count,
                syncedAgo: channel.last_synced_at
                  ? formatRelativeTime(channel.last_synced_at)
                  : null,
              })
            : null
        }
        primaryAction={
          next &&
          next.id !== selection.selectedVideo?.id && (
            <PlayLink
              compact
              to={`${channelPath(id)}?video=${encodeURIComponent(next.id)}`}
              label="Play next"
              title={next.title}
            />
          )
        }
        unwatchedCount={channel.unwatched_count}
        onSync={refreshing(() => reconcileChannel(id))}
        onMarkWatched={refreshing(() => markChannelWatched(id))}
        onEditRequest={() => setEditing(true)}
        youtubeLink={{ href: youtubeChannelUrl(id), label: 'Open channel on YouTube' }}
        aiCategorization={
          categoriesShown
            ? {
                enabled: categorizationAvailable && channel.categorization_enabled,
                onDeleteAiVerdicts: refreshing(() =>
                  deleteSourceAiVerdicts({ kind: 'channel', id }),
                ),
              }
            : undefined
        }
        onDelete={refreshing(async () => {
          await deleteSource(queryKeys.channelVideos(id), () => deleteChannel(id))
          navigate('/')
        })}
        deleteDescription="This removes the channel from tracking."
      />
      <EditChannelDialog
        channel={channel}
        open={editing}
        onOpenChange={setEditing}
        onSave={(changes) => saveChannelSettings(channel, changes)}
      />
      <WatchView
        noun="channel"
        videos={videos}
        error={error}
        selection={selection}
        basePathOf={() => channel.path}
        sourceNameOf={() => channel.name}
        detailSourceOf={() => ({
          kind: 'channel',
          name: channel.name,
          href: channelPath(id),
          avatarSrc,
        })}
        orderLabel="newest first"
      />
    </div>
  )
}
