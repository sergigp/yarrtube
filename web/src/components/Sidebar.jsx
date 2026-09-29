import { useEffect, useState } from 'react'
import { Link, useLocation, useNavigate } from 'react-router-dom'
import { CheckCheck, Ellipsis, RotateCw, Trash2, X } from 'lucide-react'
import {
  queryKeys,
  useChannels,
  useLibraryAction,
  usePlaylists,
  useRemoveQuery,
} from '../queries'
import {
  reconcileChannel,
  reconcilePlaylist,
  deleteChannel,
  deletePlaylist,
  markChannelWatched,
  avatarMediaUrl,
} from '../api'
import {
  CAUGHT_UP_PREVIEW,
  SEARCH_THRESHOLD,
  channelLeadCount,
  collapse,
  matchesSearch,
  orderChannels,
} from '../sidebarSections'
import { ConfirmDialog } from './ConfirmDialog'
import { Thumbnail } from './Thumbnail'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'

function activeIdFrom(pathname, prefix) {
  if (!pathname.startsWith(prefix)) {
    return null
  }
  const rest = pathname.slice(prefix.length).split('/')[0]
  return rest ? decodeURIComponent(rest) : null
}

/**
 * Whether a sidebar section is expanded, remembered per browser. Storage may
 * be unavailable (private mode, blocked site data): the section then starts
 * collapsed and the choice lasts until reload.
 */
function useExpandedState(section) {
  const key = `yarrtube.sidebar.expanded.${section}`
  const [expanded, setExpanded] = useState(() => {
    try {
      return window.localStorage.getItem(key) === 'true'
    } catch {
      return false
    }
  })

  const toggle = () => {
    const next = !expanded
    setExpanded(next)
    try {
      window.localStorage.setItem(key, String(next))
    } catch {
      // Not remembered; the in-memory state still applies.
    }
  }

  return [expanded, toggle]
}

/**
 * The rows a section shows: every match while searching, otherwise all rows
 * when expanded or its leading rows (plus the active one) when collapsed.
 * `collapsible` says whether collapsing would hide anything.
 */
function sectionView(items, { searchText, expanded, leadCount, activeId }) {
  if (!items) {
    return { rows: items, hiddenCount: 0, collapsible: false }
  }
  if (searchText) {
    return {
      rows: items.filter((item) => matchesSearch(item, searchText)),
      hiddenCount: 0,
      collapsible: false,
    }
  }
  const collapsed = collapse(items, leadCount, activeId)
  return {
    rows: expanded ? items : collapsed.shown,
    hiddenCount: expanded ? 0 : collapsed.hiddenCount,
    collapsible: collapsed.hiddenCount > 0,
  }
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
  hiddenCount,
  expanded,
  collapsible,
  onToggleExpanded,
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
      <h3 className="mb-2 px-2 font-heading text-base font-semibold tracking-tight text-foreground">
        {title}
      </h3>
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
      {!error && collapsible && (
        <button
          type="button"
          className="mt-1 px-2 py-1 text-xs text-muted-foreground transition-colors hover:text-foreground"
          aria-expanded={expanded}
          onClick={onToggleExpanded}
        >
          {expanded ? 'Show less' : `Show ${hiddenCount} more`}
        </button>
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
  const { data: channels, error: channelsError } = useChannels()
  const { data: playlists, error: playlistsError } = usePlaylists()
  const refreshing = useLibraryAction()
  const removeQuery = useRemoveQuery()
  const [search, setSearch] = useState('')
  const [channelsExpanded, toggleChannelsExpanded] = useExpandedState('channels')
  const [playlistsExpanded, togglePlaylistsExpanded] = useExpandedState('playlists')

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

  const searchable = (channels?.length ?? 0) + (playlists?.length ?? 0) > SEARCH_THRESHOLD
  const searchText = searchable ? search.trim() : ''
  const orderedChannels = channels && orderChannels(channels)
  const channelsView = sectionView(orderedChannels, {
    searchText,
    expanded: channelsExpanded,
    leadCount: orderedChannels && channelLeadCount(orderedChannels),
    activeId: activeChannelId,
  })
  const playlistsView = sectionView(playlists, {
    searchText,
    expanded: playlistsExpanded,
    leadCount: CAUGHT_UP_PREVIEW,
    activeId: activePlaylistId,
  })
  const showChannels = !searchText || channelsView.rows?.length !== 0
  const showPlaylists = !searchText || playlistsView.rows?.length !== 0

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
        {searchable && (
          <Input
            type="search"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Search"
            aria-label="Search channels and playlists"
          />
        )}
        {!showChannels && !showPlaylists && (
          <p className="px-2 text-sm text-muted-foreground">Nothing matches "{searchText}".</p>
        )}
        {showChannels && (
          <SidebarSection
            title="Channels"
            items={channelsView.rows}
            hiddenCount={channelsView.hiddenCount}
            expanded={channelsExpanded}
            collapsible={channelsView.collapsible}
            onToggleExpanded={toggleChannelsExpanded}
            error={channelsError}
            activeId={activeChannelId}
            hrefFor={(channel) => `/channels/${channel.id}`}
            showAvatar
            onNavigate={onClose}
            onSync={refreshing(reconcileChannel)}
            onMarkWatched={refreshing(markChannelWatched)}
            onDelete={refreshing(async (id) => {
              await deleteChannel(id)
              removeQuery(queryKeys.channelVideos(id))
              if (activeChannelId === id) {
                navigate('/')
              }
            })}
            deleteDescription="This removes the channel from tracking."
          />
        )}
        {showPlaylists && (
          <SidebarSection
            title="Playlists"
            items={playlistsView.rows}
            hiddenCount={playlistsView.hiddenCount}
            expanded={playlistsExpanded}
            collapsible={playlistsView.collapsible}
            onToggleExpanded={togglePlaylistsExpanded}
            error={playlistsError}
            activeId={activePlaylistId}
            hrefFor={(playlist) => `/playlists/${playlist.id}`}
            onNavigate={onClose}
            onSync={refreshing(reconcilePlaylist)}
            onDelete={refreshing(async (id) => {
              await deletePlaylist(id)
              removeQuery(queryKeys.playlistVideos(id))
              if (activePlaylistId === id) {
                navigate('/')
              }
            })}
            deleteDescription="This removes the playlist from tracking, along with its video records and downloaded files."
          />
        )}
      </aside>
    </>
  )
}
