import { useEffect, useState } from 'react'
import { Link, useLocation, useNavigate } from 'react-router-dom'
import { CheckCheck, Ellipsis, RotateCw, Trash2, X } from 'lucide-react'
import { usePolling } from '../usePolling'
import {
  fetchChannels,
  fetchPlaylists,
  reconcileChannel,
  reconcilePlaylist,
  deleteChannel,
  deletePlaylist,
  markChannelWatched,
  avatarMediaUrl,
} from '../api'
import { ConfirmDialog } from './ConfirmDialog'
import { Thumbnail } from './Thumbnail'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { cn } from '@/lib/utils'

function activeIdFrom(pathname, prefix) {
  if (!pathname.startsWith(prefix)) {
    return null
  }
  const rest = pathname.slice(prefix.length).split('/')[0]
  return rest ? decodeURIComponent(rest) : null
}

function SidebarRowMenu({ item, onSync, onMarkWatched, onDeleteRequest }) {
  const [syncing, setSyncing] = useState(false)

  return (
    // Non-modal so opening the delete confirmation from it doesn't leave the
    // page with pointer events disabled.
    <DropdownMenu modal={false}>
      <DropdownMenuTrigger
        className="flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground data-[state=open]:bg-secondary data-[state=open]:text-foreground"
        aria-label={`Actions for ${item.name}`}
      >
        {syncing ? (
          <RotateCw className="size-3.5 animate-spin" />
        ) : (
          <Ellipsis className="size-4" />
        )}
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-auto min-w-44">
        <DropdownMenuItem
          disabled={syncing}
          onSelect={async () => {
            setSyncing(true)
            try {
              await onSync()
            } finally {
              setSyncing(false)
            }
          }}
        >
          <RotateCw />
          Sync
        </DropdownMenuItem>
        {onMarkWatched && (
          <DropdownMenuItem onSelect={onMarkWatched}>
            <CheckCheck />
            Mark all watched
          </DropdownMenuItem>
        )}
        <DropdownMenuItem variant="destructive" onSelect={onDeleteRequest}>
          <Trash2 />
          Delete
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

function SidebarRow({
  item,
  active,
  href,
  onSync,
  onMarkWatched,
  onDeleteRequest,
  showAvatar,
  onNavigate,
}) {
  return (
    <li className={cn('flex items-center gap-1 rounded-md pr-1', active && 'bg-accent')}>
      <Link
        to={href}
        onClick={onNavigate}
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
        {item.unwatched_count > 0 && (
          <span
            className="shrink-0 rounded-full bg-primary px-1.5 py-0.5 text-[10px] leading-none font-medium text-primary-foreground"
            aria-label={`${item.unwatched_count} unwatched`}
            title={`${item.unwatched_count} unwatched`}
          >
            {item.unwatched_count}
          </span>
        )}
      </Link>
      <SidebarRowMenu
        item={item}
        onSync={onSync}
        onMarkWatched={onMarkWatched}
        onDeleteRequest={onDeleteRequest}
      />
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
  onMarkWatched,
  onDelete,
  deleteDescription,
  showAvatar,
  onNavigate,
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
              onNavigate={onNavigate}
              onSync={async () => {
                try {
                  await onSync(item.id)
                } catch (err) {
                  window.alert(`Failed to sync "${item.name}": ${err.message}`)
                }
              }}
              onMarkWatched={
                onMarkWatched &&
                (async () => {
                  try {
                    await onMarkWatched(item.id)
                  } catch (err) {
                    window.alert(`Failed to mark "${item.name}" watched: ${err.message}`)
                  }
                })
              }
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

export function Sidebar({ open = false, onClose }) {
  const location = useLocation()
  const navigate = useNavigate()
  const { data: channels, error: channelsError } = usePolling(fetchChannels, [])
  const { data: playlists, error: playlistsError } = usePolling(fetchPlaylists, [])

  // The drawer scrolls on its own; keep the page behind it still.
  useEffect(() => {
    if (!open) {
      return undefined
    }
    document.body.style.overflow = 'hidden'
    return () => {
      document.body.style.overflow = ''
    }
  }, [open])

  const activeChannelId = activeIdFrom(location.pathname, '/channels/')
  const activePlaylistId = activeIdFrom(location.pathname, '/playlists/')

  return (
    <>
      {open && (
        <div
          className="fixed inset-0 z-30 bg-black/40 md:hidden"
          onClick={onClose}
          aria-hidden="true"
        />
      )}
      <aside
        className={cn(
          'fixed inset-y-0 left-0 z-40 flex w-64 max-w-[85%] flex-col gap-6 overflow-y-auto border-r border-border bg-background px-3 py-4 shadow-lg transition-transform duration-200 ease-in-out',
          open ? 'translate-x-0' : '-translate-x-full',
          'md:static md:z-auto md:h-full md:w-60 md:max-w-none md:translate-x-0 md:shadow-none md:transition-none md:py-6',
        )}
      >
        <div className="flex items-center justify-between md:hidden">
          <span className="font-heading text-sm font-semibold text-foreground">Menu</span>
          <button
            type="button"
            className="flex size-7 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground"
            onClick={onClose}
            aria-label="Close menu"
          >
            <X className="size-4" />
          </button>
        </div>
        <SidebarSection
          title="Channels"
          items={channels}
          error={channelsError}
          activeId={activeChannelId}
          hrefFor={(channel) => `/channels/${channel.id}`}
          showAvatar
          onNavigate={onClose}
          onSync={reconcileChannel}
          onMarkWatched={markChannelWatched}
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
          onNavigate={onClose}
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
    </>
  )
}
