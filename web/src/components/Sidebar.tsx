import { useEffect, useId, useState } from 'react'
import { Link, useLocation, useNavigate } from 'react-router-dom'
import { Ellipsis, Plus, RotateCw } from 'lucide-react'
import {
  queryKeys,
  useCategories,
  useCategoriesShown,
  useChannels,
  useDeleteSource,
  useLibraryAction,
  usePlaylists,
  useSetPlaylistExcludedFromHome,
  useSetPlaylistUnwatchedCountHidden,
  useUpdatePlaylistSettings,
} from '@/api/queries'
import {
  reconcileChannel,
  reconcilePlaylist,
  deleteChannel,
  deletePlaylist,
  markChannelWatched,
  markPlaylistWatched,
  avatarMediaUrl,
} from '@/api/client'
import type { LibraryItem } from '@/api/types'
import {
  activeIdFrom,
  orderByUnwatched,
  rowCountSlotClassName,
  rowMenuClassName,
  sectionView,
  shownUnwatchedCount,
  unwatchedTotal,
} from '@/lib/sidebarSections'
import { markWatchedConfirmation } from '@/lib/markWatchedConfirmation'
import { deleteSourceDescription } from '@/lib/sourceDelete'
import { ConfirmDialog } from './ConfirmDialog'
import { EditChannelDialog } from './EditChannelDialog'
import { EditPlaylistDialog } from './EditPlaylistDialog'
import { EntryActionsMenu } from './EntryActionsMenu'
import { SidebarCategories } from './SidebarCategories'
import { FAVORITES_PATH, SidebarFavorites } from './SidebarFavorites'
import { SidebarCount } from './SidebarCount'
import { SidebarFilter } from './SidebarFilter'
import { PlaylistIcon } from './PlaylistIcon'
import { SIDEBAR_ROW_CLASS, ShowMoreButton, sidebarRowLinkClassName } from './SidebarRowParts'
import { SidebarSectionHeading } from './SidebarSectionHeading'
import { Thumbnail } from './Thumbnail'
import { DropdownMenuItem } from '@/components/ui/dropdown-menu'
import { useRememberedFlag } from '@/hooks/useRememberedFlag'
import { useSaveChannelSettings } from '@/hooks/useSaveChannelSettings'
import { errorMessage } from '@/lib/errorMessage'
import { cn } from '@/lib/utils'
import { channelPath } from '@/lib/sourcePath'
import { matchesSearch } from '@/lib/search'

interface SidebarRowMenuProps {
  item: LibraryItem
  className?: string
  onSync: () => Promise<void>
  onMarkWatched?: (() => Promise<void>) | undefined
  onSetExcludedFromHome?: ((excluded: boolean) => Promise<void>) | undefined
  onSetUnwatchedCountHidden?: ((hidden: boolean) => Promise<void>) | undefined
  onEditRequest?: (() => void) | undefined
  onDeleteRequest: () => void
}

function SidebarRowMenu({
  item,
  className,
  onSync,
  onMarkWatched,
  onSetExcludedFromHome,
  onSetUnwatchedCountHidden,
  onEditRequest,
  onDeleteRequest,
}: SidebarRowMenuProps) {
  const [syncing, setSyncing] = useState(false)

  return (
    <EntryActionsMenu
      name={item.name}
      className={className}
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
      unwatchedCountHidden={onSetUnwatchedCountHidden && (item.hide_unwatched_count ?? false)}
      onSetUnwatchedCountHidden={onSetUnwatchedCountHidden}
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
  onSetUnwatchedCountHidden?: ((hidden: boolean) => Promise<void>) | undefined
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
  onSetUnwatchedCountHidden,
  onEditRequest,
  onDeleteRequest,
  showAvatar,
  onNavigate,
}: SidebarRowProps) {
  const unwatchedCount = shownUnwatchedCount(item)
  return (
    <li className={cn(SIDEBAR_ROW_CLASS, active && 'bg-accent')}>
      <Link
        to={href}
        onClick={onNavigate}
        aria-current={active ? 'page' : undefined}
        className={sidebarRowLinkClassName(active)}
      >
        {showAvatar ? (
          <Thumbnail
            src={item.avatar_filename ? avatarMediaUrl(item.avatar_filename) : null}
            className="size-5 shrink-0 rounded-full object-cover"
          />
        ) : (
          <PlaylistIcon />
        )}
        <span className="min-w-0 truncate">{item.name}</span>
        <span className={rowCountSlotClassName()}>
          {unwatchedCount > 0 && <SidebarCount count={unwatchedCount} active={active} />}
        </span>
      </Link>
      <SidebarRowMenu
        item={item}
        className={rowMenuClassName()}
        onSync={onSync}
        onMarkWatched={onMarkWatched}
        onSetExcludedFromHome={onSetExcludedFromHome}
        onSetUnwatchedCountHidden={onSetUnwatchedCountHidden}
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
  folded: boolean
  headingActive: boolean
  onToggleFolded: () => void
  /** Channels only; omitted for Playlists. Shown when folded and > 0. */
  foldedTotal?: number | undefined
  error: Error | null
  activeId: string | null
  hrefFor: (item: LibraryItem) => string
  onSync: (id: string) => Promise<void>
  onMarkWatched?: (id: string) => Promise<void>
  onSetExcludedFromHome?: (id: string, excluded: boolean) => Promise<void>
  onSetUnwatchedCountHidden?: (id: string, hidden: boolean) => Promise<void>
  /** Channels only: opens the edit channel dialog, which the caller owns. */
  onEditRequest?: (id: string) => void
  onDelete: (id: string) => Promise<void>
  /** What deleting `item` does, for its delete confirmation. */
  deleteDescriptionOf: (item: LibraryItem) => string
  showAvatar?: boolean
  onNavigate: () => void
  /** Rows left out past the section's limit; offers "Show N more" when > 0. */
  hiddenCount: number
  onShowMore: () => void
  /** Filtering: a section with no match hides, and an empty one says so no more. */
  filtering: boolean
}

function SidebarSection({
  title,
  addLabel,
  onAdd,
  onClose,
  items,
  folded,
  headingActive,
  onToggleFolded,
  foldedTotal,
  error,
  activeId,
  hrefFor,
  onSync,
  onMarkWatched,
  onSetExcludedFromHome,
  onSetUnwatchedCountHidden,
  onEditRequest,
  onDelete,
  deleteDescriptionOf,
  showAvatar,
  onNavigate,
  hiddenCount,
  onShowMore,
  filtering,
}: SidebarSectionProps) {
  const [pendingDelete, setPendingDelete] = useState<LibraryItem | null>(null)
  const [pendingMarkWatched, setPendingMarkWatched] = useState<LibraryItem | null>(null)
  const markWatchedPrompt =
    pendingMarkWatched &&
    markWatchedConfirmation(pendingMarkWatched.name, pendingMarkWatched.unwatched_count ?? 0)
  const headingId = useId()
  // Closing first lets the mobile drawer get out of the dialog's way;
  // on desktop the sidebar is static and `onClose` changes nothing.
  const add = () => {
    onClose()
    onAdd()
  }
  // An empty section gets a full-width add row; otherwise a compact `+` on the heading.
  const empty = !error && items?.length === 0
  if (filtering && (error || !items || items.length === 0)) {
    return null
  }

  return (
    <section aria-labelledby={headingId}>
      <SidebarSectionHeading
        id={headingId}
        title={title}
        folded={folded}
        onToggleFolded={onToggleFolded}
        headingActive={headingActive}
        foldedTotal={foldedTotal}
        add={empty ? undefined : { label: addLabel, onClick: add }}
      />
      {!folded && (
        <>
          {empty && (
            <button
              type="button"
              className="mb-1 flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground"
              onClick={add}
            >
              <Plus className="size-4 shrink-0" />
              {addLabel}
            </button>
          )}
          {error && (
            <p className="px-2 text-sm text-destructive">Failed to load: {error.message}</p>
          )}
          {!error && !items && <p className="px-2 text-sm text-muted-foreground">Loading…</p>}
          {!error && items && items.length === 0 && (
            <p className="px-2 text-sm text-muted-foreground">None tracked yet.</p>
          )}
          {!error && items && items.length > 0 && (
            <ul className="flex flex-col" aria-labelledby={headingId}>
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
                  onMarkWatched={
                    onMarkWatched &&
                    (async () => {
                      // No undo: always confirmed first.
                      setPendingMarkWatched(item)
                    })
                  }
                  onSetExcludedFromHome={
                    onSetExcludedFromHome &&
                    ((excluded: boolean) => onSetExcludedFromHome(item.id, excluded))
                  }
                  onSetUnwatchedCountHidden={
                    onSetUnwatchedCountHidden &&
                    ((hidden: boolean) => onSetUnwatchedCountHidden(item.id, hidden))
                  }
                  onEditRequest={onEditRequest && (() => onEditRequest(item.id))}
                  onDeleteRequest={() => setPendingDelete(item)}
                />
              ))}
            </ul>
          )}
          {hiddenCount > 0 && <ShowMoreButton count={hiddenCount} onClick={onShowMore} />}
        </>
      )}

      <ConfirmDialog
        open={pendingDelete !== null}
        onOpenChange={(open) => {
          if (!open) {
            setPendingDelete(null)
          }
        }}
        title={pendingDelete ? `Delete "${pendingDelete.name}"?` : ''}
        description={pendingDelete ? deleteDescriptionOf(pendingDelete) : undefined}
        onConfirm={async () => {
          if (pendingDelete) {
            await onDelete(pendingDelete.id)
          }
        }}
      />
      <ConfirmDialog
        open={markWatchedPrompt !== null}
        onOpenChange={(open) => {
          if (!open) {
            setPendingMarkWatched(null)
          }
        }}
        title={markWatchedPrompt?.title ?? ''}
        description={markWatchedPrompt?.description}
        confirmLabel="Mark watched"
        pendingLabel="Marking…"
        onConfirm={async () => {
          if (pendingMarkWatched && onMarkWatched) {
            await onMarkWatched(pendingMarkWatched.id)
          }
        }}
      />
    </section>
  )
}

export const SIDEBAR_ID = 'app-sidebar'

/** Rows a section shows before "Show N more", most unwatched first. */
const CHANNELS_SHOWN = 8
const PLAYLISTS_SHOWN = 6

interface SidebarProps {
  open?: boolean
  /** Viewport offset the mobile drawer and its backdrop start at, below the header. */
  top?: number
  onClose: () => void
  onAddChannel: () => void
  onAddPlaylist: () => void
  onAddCategory: () => void
}

export function Sidebar({
  open = false,
  top = 0,
  onClose,
  onAddChannel,
  onAddPlaylist,
  onAddCategory,
}: SidebarProps) {
  const location = useLocation()
  const navigate = useNavigate()
  const { data: channels, error: channelsError } = useChannels()
  const { data: playlists, error: playlistsError } = usePlaylists()
  const withCategories = useCategoriesShown()
  const refreshing = useLibraryAction()
  const deleteSource = useDeleteSource()
  const setPlaylistExcludedFromHome = useSetPlaylistExcludedFromHome()
  const setPlaylistUnwatchedCountHidden = useSetPlaylistUnwatchedCountHidden()
  const saveChannelSettings = useSaveChannelSettings()
  const [editingChannelId, setEditingChannelId] = useState<string | null>(null)
  const editingChannel = channels?.find((channel) => channel.id === editingChannelId) ?? null
  const updatePlaylistSettings = useUpdatePlaylistSettings()
  const [editingPlaylistId, setEditingPlaylistId] = useState<string | null>(null)
  const editingPlaylist =
    playlists?.find((playlist) => playlist.id === editingPlaylistId) ?? null
  const [channelsFolded, toggleChannelsFolded] = useRememberedFlag(
    'yarrtube.sidebar.folded.channels',
    false,
  )
  const [playlistsFolded, togglePlaylistsFolded] = useRememberedFlag(
    'yarrtube.sidebar.folded.playlists',
    false,
  )
  const [filterText, setFilterText] = useState('')
  const filter = filterText.trim()
  // Only to tell whether any category matches; SidebarCategories fetches the list.
  const { data: categoryList } = useCategories({ enabled: withCategories && filter !== '' })
  const [channelsExpanded, setChannelsExpanded] = useState(false)
  const [playlistsExpanded, setPlaylistsExpanded] = useState(false)

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

  const orderedChannels = channels && orderByUnwatched(channels)
  const channelsView = sectionView(orderedChannels, {
    folded: channelsFolded,
    activeId: activeChannelId,
    filter,
    limit: CHANNELS_SHOWN,
    expanded: channelsExpanded,
  })
  const playlistsView = sectionView(playlists && orderByUnwatched(playlists), {
    folded: playlistsFolded,
    activeId: activePlaylistId,
    filter,
    limit: PLAYLISTS_SHOWN,
    expanded: playlistsExpanded,
  })
  const categoryMatches = withCategories
    ? (categoryList?.categories ?? []).filter((category) => matchesSearch(category, filter))
    : []
  const noMatches =
    filter !== '' &&
    categoryMatches.length === 0 &&
    !channelsView.rows?.length &&
    !playlistsView.rows?.length

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
          'fixed bottom-0 left-0 z-40 flex w-64 max-w-[85%] flex-col overflow-y-auto border-r border-border bg-background p-3 shadow-lg transition-transform duration-200 ease-in-out',
          open ? 'translate-x-0' : '-translate-x-full',
          'md:static md:z-auto md:h-full md:w-62 md:max-w-none md:translate-x-0 md:shadow-none md:transition-none',
          // A rule between sections, once each is drawn.
          '[&>section+section]:mt-3 [&>section+section]:border-t [&>section+section]:border-border [&>section+section]:pt-3',
        )}
      >
        <SidebarFilter
          value={filterText}
          onChange={setFilterText}
          withCategories={withCategories}
        />
        {noMatches && <p className="px-3 py-1 text-sm text-muted-foreground">No matches.</p>}
        {filter === '' && (
          <SidebarFavorites active={location.pathname === FAVORITES_PATH} onNavigate={onClose} />
        )}
        {withCategories && (
          <SidebarCategories onAddCategory={onAddCategory} onClose={onClose} filter={filter} />
        )}
        <SidebarSection
          title="Channels"
          addLabel="Add channel"
          onAdd={onAddChannel}
          onClose={onClose}
          items={channelsView.rows}
          folded={channelsView.folded}
          headingActive={channelsView.headingActive}
          onToggleFolded={toggleChannelsFolded}
          foldedTotal={channels && unwatchedTotal(channels)}
          error={channelsError}
          activeId={activeChannelId}
          hrefFor={(channel) => channelPath(channel.id)}
          showAvatar
          onNavigate={onClose}
          hiddenCount={channelsView.hiddenCount}
          onShowMore={() => setChannelsExpanded(true)}
          filtering={filter !== ''}
          onSync={refreshing(reconcileChannel)}
          onMarkWatched={refreshing(markChannelWatched)}
          onEditRequest={setEditingChannelId}
          onDelete={refreshing(async (id: string) => {
            await deleteSource(queryKeys.channelVideos(id), () => deleteChannel(id))
            if (activeChannelId === id) {
              navigate('/')
            }
          })}
          deleteDescriptionOf={(channel) =>
            deleteSourceDescription('channel', channel.favorite_count ?? 0)
          }
        />
        <SidebarSection
          title="Playlists"
          addLabel="Add playlist"
          onAdd={onAddPlaylist}
          onClose={onClose}
          items={playlistsView.rows}
          folded={playlistsView.folded}
          headingActive={playlistsView.headingActive}
          onToggleFolded={togglePlaylistsFolded}
          error={playlistsError}
          activeId={activePlaylistId}
          hrefFor={(playlist) => `/playlists/${playlist.id}`}
          onNavigate={onClose}
          hiddenCount={playlistsView.hiddenCount}
          onShowMore={() => setPlaylistsExpanded(true)}
          filtering={filter !== ''}
          onSync={refreshing(reconcilePlaylist)}
          onMarkWatched={refreshing(markPlaylistWatched)}
          onSetExcludedFromHome={setPlaylistExcludedFromHome}
          onSetUnwatchedCountHidden={setPlaylistUnwatchedCountHidden}
          onEditRequest={setEditingPlaylistId}
          onDelete={refreshing(async (id: string) => {
            await deleteSource(queryKeys.playlistVideos(id), () => deletePlaylist(id))
            if (activePlaylistId === id) {
              navigate('/')
            }
          })}
          deleteDescriptionOf={(playlist) =>
            deleteSourceDescription('playlist', playlist.favorite_count ?? 0)
          }
        />
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
      {editingPlaylist && (
        <EditPlaylistDialog
          playlist={editingPlaylist}
          open
          onOpenChange={(isOpen) => {
            if (!isOpen) {
              setEditingPlaylistId(null)
            }
          }}
          onSave={(changes) => updatePlaylistSettings(editingPlaylist.id, changes)}
        />
      )}
    </>
  )
}
