import { useEffect, useState } from 'react'
import { Link, useLocation, useNavigate } from 'react-router-dom'
import { Ellipsis, Plus, RotateCw } from 'lucide-react'
import {
  queryKeys,
  useChannels,
  useLibraryAction,
  usePlaylists,
  useRemoveQuery,
  useSetPlaylistExcludedFromHome,
} from '@/api/queries'
import {
  reconcileChannel,
  reconcilePlaylist,
  deleteChannel,
  deletePlaylist,
  markChannelWatched,
  avatarMediaUrl,
} from '@/api/client'
import type { LibraryItem } from '@/api/types'
import {
  CAUGHT_UP_PREVIEW,
  SEARCH_THRESHOLD,
  channelLeadCount,
  collapse,
  matchesSearch,
  orderChannels,
} from '@/lib/sidebarSections'
import { ConfirmDialog } from './ConfirmDialog'
import { EditChannelDialog } from './EditChannelDialog'
import { EntryActionsMenu } from './EntryActionsMenu'
import { Thumbnail } from './Thumbnail'
import { DropdownMenuItem } from '@/components/ui/dropdown-menu'
import { Input } from '@/components/ui/input'
import { useSaveChannelSettings } from '@/hooks/useSaveChannelSettings'
import { errorMessage } from '@/lib/errorMessage'
import { cn } from '@/lib/utils'

function activeIdFrom(pathname: string, prefix: string): string | null {
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
function useExpandedState(section: string): readonly [boolean, () => void] {
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

  return [expanded, toggle] as const
}

interface SectionViewOptions {
  searchText: string
  expanded: boolean
  leadCount: number
  activeId: string | null
}

interface SectionView {
  rows: LibraryItem[] | undefined
  hiddenCount: number
  collapsible: boolean
}

/**
 * The rows a section shows: every match while searching, otherwise all rows
 * when expanded or its leading rows (plus the active one) when collapsed.
 * `collapsible` says whether collapsing would hide anything.
 */
function sectionView(
  items: LibraryItem[] | undefined,
  { searchText, expanded, leadCount, activeId }: SectionViewOptions,
): SectionView {
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

interface SidebarRowMenuProps {
  item: LibraryItem
  onSync: () => Promise<void>
  onMarkWatched?: (() => Promise<void>) | undefined
  onSetExcludedFromHome?: ((excluded: boolean) => Promise<void>) | undefined
  onEditRequest?: (() => void) | undefined
  onDeleteRequest: () => void
}

function SidebarRowMenu({
  item,
  onSync,
  onMarkWatched,
  onSetExcludedFromHome,
  onEditRequest,
  onDeleteRequest,
}: SidebarRowMenuProps) {
  const [syncing, setSyncing] = useState(false)

  return (
    <EntryActionsMenu
      name={item.name}
      triggerIcon={
        syncing ? <RotateCw className="size-3.5 animate-spin" /> : <Ellipsis className="size-4" />
      }
      leadingItems={
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
      }
      onMarkWatched={onMarkWatched}
      excludedFromHome={onSetExcludedFromHome && (item.exclude_from_home ?? false)}
      onSetExcludedFromHome={onSetExcludedFromHome}
      onEditRequest={onEditRequest}
      onDeleteRequest={onDeleteRequest}
    />
  )
}

interface SidebarRowProps {
  item: LibraryItem
  active: boolean
  href: string
  onSync: () => Promise<void>
  onMarkWatched?: (() => Promise<void>) | undefined
  onSetExcludedFromHome?: ((excluded: boolean) => Promise<void>) | undefined
  onEditRequest?: (() => void) | undefined
  onDeleteRequest: () => void
  showAvatar?: boolean | undefined
  onNavigate: () => void
}

function SidebarRow({
  item,
  active,
  href,
  onSync,
  onMarkWatched,
  onSetExcludedFromHome,
  onEditRequest,
  onDeleteRequest,
  showAvatar,
  onNavigate,
}: SidebarRowProps) {
  const unwatchedCount = item.unwatched_count ?? 0
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
        {unwatchedCount > 0 && (
          <span
            className="shrink-0 rounded-full bg-primary px-1.5 py-0.5 text-[10px] leading-none font-medium text-primary-foreground"
            aria-label={`${unwatchedCount} unwatched`}
            title={`${unwatchedCount} unwatched`}
          >
            {unwatchedCount}
          </span>
        )}
      </Link>
      <SidebarRowMenu
        item={item}
        onSync={onSync}
        onMarkWatched={onMarkWatched}
        onSetExcludedFromHome={onSetExcludedFromHome}
        onEditRequest={onEditRequest}
        onDeleteRequest={onDeleteRequest}
      />
    </li>
  )
}

interface SidebarSectionProps {
  title: string
  addLabel: string
  onAdd: () => void
  onClose: () => void
  items: LibraryItem[] | undefined
  hiddenCount: number
  expanded: boolean
  collapsible: boolean
  onToggleExpanded: () => void
  error: Error | null
  activeId: string | null
  hrefFor: (item: LibraryItem) => string
  onSync: (id: string) => Promise<void>
  onMarkWatched?: (id: string) => Promise<void>
  onSetExcludedFromHome?: (id: string, excluded: boolean) => Promise<void>
  /** Channels only: opens the edit channel dialog, which the caller owns. */
  onEditRequest?: (id: string) => void
  onDelete: (id: string) => Promise<void>
  deleteDescription: string
  showAvatar?: boolean
  onNavigate: () => void
}

function SidebarSection({
  title,
  addLabel,
  onAdd,
  onClose,
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
  onSetExcludedFromHome,
  onEditRequest,
  onDelete,
  deleteDescription,
  showAvatar,
  onNavigate,
}: SidebarSectionProps) {
  const [pendingDelete, setPendingDelete] = useState<LibraryItem | null>(null)

  return (
    <div>
      <h3 className="mb-2 px-2 font-heading text-base font-semibold tracking-tight text-foreground">
        {title}
      </h3>
      {/* Closing first lets the mobile drawer get out of the dialog's way;
          on desktop the sidebar is static and `onClose` changes nothing. */}
      <button
        type="button"
        className="mb-1 flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground"
        onClick={() => {
          onClose()
          onAdd()
        }}
      >
        <Plus className="size-4 shrink-0" />
        {addLabel}
      </button>
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
                  window.alert(`Failed to sync "${item.name}": ${errorMessage(err)}`)
                }
              }}
              onMarkWatched={onMarkWatched && (() => onMarkWatched(item.id))}
              onSetExcludedFromHome={
                onSetExcludedFromHome &&
                ((excluded: boolean) => onSetExcludedFromHome(item.id, excluded))
              }
              onEditRequest={onEditRequest && (() => onEditRequest(item.id))}
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
          if (pendingDelete) {
            await onDelete(pendingDelete.id)
          }
        }}
      />
    </div>
  )
}

export const SIDEBAR_ID = 'app-sidebar'

interface SidebarProps {
  open?: boolean
  /** Viewport offset the mobile drawer and its backdrop start at, below the header. */
  top?: number
  onClose: () => void
  onAddChannel: () => void
  onAddPlaylist: () => void
}

export function Sidebar({
  open = false,
  top = 0,
  onClose,
  onAddChannel,
  onAddPlaylist,
}: SidebarProps) {
  const location = useLocation()
  const navigate = useNavigate()
  const { data: channels, error: channelsError } = useChannels()
  const { data: playlists, error: playlistsError } = usePlaylists()
  const refreshing = useLibraryAction()
  const removeQuery = useRemoveQuery()
  const setPlaylistExcludedFromHome = useSetPlaylistExcludedFromHome()
  const saveChannelSettings = useSaveChannelSettings()
  const [editingChannelId, setEditingChannelId] = useState<string | null>(null)
  const editingChannel = channels?.find((channel) => channel.id === editingChannelId) ?? null
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

  // Escape already consumed by an open row menu or dialog (Radix prevents
  // default) dismisses only that layer, not the drawer behind it.
  useEffect(() => {
    if (!open) {
      return undefined
    }
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && !event.defaultPrevented) {
        onClose()
      }
    }
    document.addEventListener('keydown', closeOnEscape)
    return () => {
      document.removeEventListener('keydown', closeOnEscape)
    }
  }, [open, onClose])

  const activeChannelId = activeIdFrom(location.pathname, '/channels/')
  const activePlaylistId = activeIdFrom(location.pathname, '/playlists/')

  const searchable = (channels?.length ?? 0) + (playlists?.length ?? 0) > SEARCH_THRESHOLD
  const searchText = searchable ? search.trim() : ''
  const orderedChannels = channels && orderChannels(channels)
  const channelsView = sectionView(orderedChannels, {
    searchText,
    expanded: channelsExpanded,
    leadCount: orderedChannels ? channelLeadCount(orderedChannels) : 0,
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
          className="fixed inset-x-0 bottom-0 z-30 bg-black/40 md:hidden"
          style={{ top }}
          onClick={onClose}
          aria-hidden="true"
        />
      )}
      <aside
        id={SIDEBAR_ID}
        style={{ top }}
        className={cn(
          'fixed bottom-0 left-0 z-40 flex w-64 max-w-[85%] flex-col gap-6 overflow-y-auto border-r border-border bg-background px-3 py-4 shadow-lg transition-transform duration-200 ease-in-out',
          open ? 'translate-x-0' : '-translate-x-full',
          'md:static md:z-auto md:h-full md:w-60 md:max-w-none md:translate-x-0 md:shadow-none md:transition-none md:py-6',
        )}
      >
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
            addLabel="Add channel"
            onAdd={onAddChannel}
            onClose={onClose}
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
            onEditRequest={setEditingChannelId}
            onDelete={refreshing(async (id: string) => {
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
            addLabel="Add playlist"
            onAdd={onAddPlaylist}
            onClose={onClose}
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
            onSetExcludedFromHome={setPlaylistExcludedFromHome}
            onDelete={refreshing(async (id: string) => {
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
      {editingChannel && (
        <EditChannelDialog
          channel={editingChannel}
          open
          onOpenChange={(isOpen) => {
            if (!isOpen) {
              setEditingChannelId(null)
            }
          }}
          onSave={(changes) => saveChannelSettings(editingChannel, changes)}
        />
      )}
    </>
  )
}
