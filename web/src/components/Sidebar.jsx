import { useState } from 'react'
import { Link, useLocation, useNavigate } from 'react-router-dom'
import { RotateCw, X } from 'lucide-react'
import { usePolling } from '../usePolling'
import {
  fetchChannels,
  fetchPlaylists,
  reconcileChannel,
  reconcilePlaylist,
  deleteChannel,
  deletePlaylist,
  avatarMediaUrl,
} from '../api'
import { ConfirmDialog } from './ConfirmDialog'
import { Thumbnail } from './Thumbnail'
import { cn } from '@/lib/utils'

function activeIdFrom(pathname, prefix) {
  if (!pathname.startsWith(prefix)) {
    return null
  }
  const rest = pathname.slice(prefix.length).split('/')[0]
  return rest ? decodeURIComponent(rest) : null
}

function SidebarRow({ item, active, href, onSync, onDeleteRequest, showAvatar }) {
  const [syncing, setSyncing] = useState(false)

  return (
    <li className={cn('group flex items-center rounded-md', active && 'bg-accent')}>
      <Link
        to={href}
        className={cn(
          'flex min-w-0 flex-1 items-center gap-2 truncate px-2 py-1.5 text-sm no-underline transition-colors',
          active ? 'font-medium text-primary' : 'text-foreground hover:text-primary',
        )}
      >
        {showAvatar && (
          <Thumbnail
            src={item.avatar_filename ? avatarMediaUrl(item.avatar_filename) : null}
            className="size-5 shrink-0 rounded-full object-cover"
          />
        )}
        <span className="min-w-0 flex-1 truncate">{item.name}</span>
      </Link>
      <span className="flex shrink-0 items-center gap-0.5 pr-1 opacity-0 transition-opacity focus-within:opacity-100 group-hover:opacity-100">
        <button
          type="button"
          className="flex size-6 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground disabled:opacity-50"
          onClick={async () => {
            setSyncing(true)
            try {
              await onSync()
            } finally {
              setSyncing(false)
            }
          }}
          disabled={syncing}
          aria-label={`Sync ${item.name}`}
          title="Sync"
        >
          <RotateCw className={cn('size-3.5', syncing && 'animate-spin')} />
        </button>
        <button
          type="button"
          className="flex size-6 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-destructive/10 hover:text-destructive"
          onClick={onDeleteRequest}
          aria-label={`Delete ${item.name}`}
          title="Delete"
        >
          <X className="size-3.5" />
        </button>
      </span>
    </li>
  )
}

function SidebarSection({
  title,
  items,
  error,
  activeId,
  hrefFor,
  onSync,
  onDelete,
  deleteDescription,
  showAvatar,
}) {
  const [pendingDelete, setPendingDelete] = useState(null)

  return (
    <div>
      <h3 className="mb-1 px-2 text-sm font-medium text-muted-foreground">{title}</h3>
      {error && <p className="px-2 text-sm text-destructive">Failed to load: {error.message}</p>}
      {!error && !items && <p className="px-2 text-sm text-muted-foreground">Loading…</p>}
      {!error && items && items.length === 0 && (
        <p className="px-2 text-sm text-muted-foreground">None tracked yet.</p>
      )}
      {!error && items && items.length > 0 && (
        <ul className="flex flex-col">
          {items.map((item) => (
            <SidebarRow
              key={item.id}
              item={item}
              active={item.id === activeId}
              href={hrefFor(item)}
              showAvatar={showAvatar}
              onSync={async () => {
                try {
                  await onSync(item.id)
                } catch (err) {
                  window.alert(`Failed to sync "${item.name}": ${err.message}`)
                }
              }}
              onDeleteRequest={() => setPendingDelete(item)}
            />
          ))}
        </ul>
      )}

      <ConfirmDialog
        open={pendingDelete !== null}
        onOpenChange={(open) => {
          if (!open) {
            setPendingDelete(null)
          }
        }}
        title={pendingDelete ? `Delete "${pendingDelete.name}"?` : ''}
        description={deleteDescription}
        onConfirm={async () => {
          await onDelete(pendingDelete.id)
        }}
      />
    </div>
  )
}

export function Sidebar() {
  const location = useLocation()
  const navigate = useNavigate()
  const { data: channels, error: channelsError } = usePolling(fetchChannels, [])
  const { data: playlists, error: playlistsError } = usePolling(fetchPlaylists, [])

  const activeChannelId = activeIdFrom(location.pathname, '/channels/')
  const activePlaylistId = activeIdFrom(location.pathname, '/playlists/')

  return (
    <aside className="flex w-full shrink-0 flex-col gap-6 overflow-y-auto border-b border-border px-3 py-4 md:h-full md:w-60 md:border-r md:border-b-0 md:py-6">
      <SidebarSection
        title="Channels"
        items={channels}
        error={channelsError}
        activeId={activeChannelId}
        hrefFor={(channel) => `/channels/${channel.id}`}
        showAvatar
        onSync={reconcileChannel}
        onDelete={async (id) => {
          await deleteChannel(id)
          if (activeChannelId === id) {
            navigate('/')
          }
        }}
        deleteDescription="This removes the channel from tracking."
      />
      <SidebarSection
        title="Playlists"
        items={playlists}
        error={playlistsError}
        activeId={activePlaylistId}
        hrefFor={(playlist) => `/playlists/${playlist.id}`}
        onSync={reconcilePlaylist}
        onDelete={async (id) => {
          await deletePlaylist(id)
          if (activePlaylistId === id) {
            navigate('/')
          }
        }}
        deleteDescription="This removes the playlist from tracking, along with its video records and downloaded files."
      />
    </aside>
  )
}
