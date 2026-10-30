import { useState, type ReactNode } from 'react'
import {
  Bell,
  BellOff,
  CheckCheck,
  EllipsisVertical,
  Eye,
  EyeOff,
  RotateCw,
  SlidersHorizontal,
  Trash2,
} from 'lucide-react'
import { ConfirmDialog } from './ConfirmDialog'
import { EditableTitle } from './EditableTitle'
import { Thumbnail } from './Thumbnail'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { useIsDesktop } from '@/hooks/useIsDesktop'
import { errorMessage } from '@/lib/errorMessage'
import { homeSettingAction } from '@/lib/homeSetting'
import { markWatchedConfirmation } from '@/lib/markWatchedConfirmation'
import { unwatchedCountSettingAction } from '@/lib/unwatchedCountSetting'
import { cn } from '@/lib/utils'

function summaryFor(
  videos: readonly unknown[] | undefined,
  unwatchedCount: number | undefined,
  prefix: string | undefined,
): string | null {
  if (!videos) {
    return null
  }
  const parts = [`${videos.length} ${videos.length === 1 ? 'video' : 'videos'}`]
  if (prefix) {
    parts.unshift(prefix)
  }
  if (unwatchedCount !== undefined && unwatchedCount > 0) {
    parts.push(`${unwatchedCount} unwatched`)
  }
  return parts.join(' · ')
}

/** One page-level action, rendered as a button on desktop and a menu item on mobile. */
export interface HeaderAction {
  label: string
  icon: ReactNode
  onSelect: () => void
  disabled?: boolean
}

/** Runs `action`, alerting `failure: <error>` when it throws. */
async function alertOnFailure(action: () => Promise<void>, failure: string): Promise<void> {
  try {
    await action()
  } catch (err) {
    window.alert(`${failure}: ${errorMessage(err)}`)
  }
}

/** A square, icon-only button naming its action in a tooltip and its accessible name. */
function IconAction({ action }: { action: HeaderAction }) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          variant="outline"
          size="icon"
          aria-label={action.label}
          disabled={action.disabled}
          onClick={action.onSelect}
        >
          {action.icon}
        </Button>
      </TooltipTrigger>
      <TooltipContent>{action.label}</TooltipContent>
    </Tooltip>
  )
}

/**
 * The "⋮" menu: on desktop it holds only delete, after the icon buttons; on
 * mobile every action, with delete set apart. Non-modal so opening the delete
 * confirmation from it doesn't leave the page with pointer events disabled.
 * Labelled apart from the sidebar row's "Actions for <name>", which is on
 * screen at the same time.
 */
function MoreActionsMenu({
  name,
  actions,
  onDelete,
  desktop = false,
}: {
  name: string
  actions: HeaderAction[]
  onDelete: () => void
  desktop?: boolean
}) {
  return (
    <DropdownMenu modal={false}>
      {desktop ? (
        <DropdownMenuTrigger asChild>
          <Button variant="outline" size="icon" aria-label={`More actions for ${name}`}>
            <EllipsisVertical />
          </Button>
        </DropdownMenuTrigger>
      ) : (
        <DropdownMenuTrigger
          className="flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground data-[state=open]:bg-secondary data-[state=open]:text-foreground"
          aria-label={`More actions for ${name}`}
        >
          <EllipsisVertical className="size-4" />
        </DropdownMenuTrigger>
      )}
      <DropdownMenuContent align="end" className="w-auto min-w-44">
        {actions.map((action) => (
          <DropdownMenuItem
            key={action.label}
            disabled={action.disabled}
            onSelect={action.onSelect}
          >
            {action.icon}
            {action.label}
          </DropdownMenuItem>
        ))}
        {actions.length > 0 && <DropdownMenuSeparator />}
        <DropdownMenuItem variant="destructive" onSelect={onDelete}>
          <Trash2 />
          Delete
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

interface DetailHeaderProps {
  name: string
  /** Background color of the whole header, e.g. a category's color. */
  tint?: string
  /** Shown before the name, e.g. a category's color control. */
  colorControl?: ReactNode
  avatarSrc?: string | null
  showAvatar?: boolean
  videos: readonly unknown[] | undefined
  unwatchedCount?: number
  /** Shown before the video counts, e.g. "2 sources". */
  summaryPrefix?: string
  /** Shows the Sync action; categories have none. */
  onSync?: () => Promise<void>
  /** Shows a "Rename" pencil that edits the name in place. */
  onRename?: (name: string) => Promise<void>
  /** Actions listed right after Sync, e.g. a category's "Manage sources". */
  extraActions?: HeaderAction[]
  onMarkWatched?: () => Promise<void>
  /** Playlists only: whether the playlist is excluded from home. */
  excludedFromHome?: boolean
  onSetExcludedFromHome?: (excluded: boolean) => Promise<void>
  /** Playlists only: whether the playlist's unwatched count is hidden. */
  unwatchedCountHidden?: boolean
  onSetUnwatchedCountHidden?: (hidden: boolean) => Promise<void>
  /** Opens the edit channel or playlist dialog, which the caller owns. */
  onEditRequest?: () => void
  onDelete: () => Promise<void>
  deleteDescription: string
}

/**
 * Title block at the top of a channel/playlist/category page: optional
 * avatar, the name, a video/unwatched summary and the page-level actions
 * (sync, mark all watched, settings, delete behind a confirmation). Desktop
 * lays every action but delete out as an icon button named in its tooltip,
 * then a "⋮" offering delete; mobile keeps them all behind one "⋮".
 */
export function DetailHeader({
  name,
  tint,
  colorControl,
  avatarSrc,
  showAvatar,
  videos,
  unwatchedCount,
  summaryPrefix,
  onSync,
  onRename,
  extraActions = [],
  onMarkWatched,
  excludedFromHome,
  onSetExcludedFromHome,
  unwatchedCountHidden,
  onSetUnwatchedCountHidden,
  onEditRequest,
  onDelete,
  deleteDescription,
}: DetailHeaderProps) {
  const isDesktop = useIsDesktop()
  const [syncing, setSyncing] = useState(false)
  const [markingWatched, setMarkingWatched] = useState(false)
  const [confirmingDelete, setConfirmingDelete] = useState(false)
  const [confirmingMarkWatched, setConfirmingMarkWatched] = useState(false)
  const markWatchedPrompt = markWatchedConfirmation(name, unwatchedCount ?? 0)
  const summary = summaryFor(videos, unwatchedCount, summaryPrefix)

  const actions: HeaderAction[] = []
  if (onSync) {
    actions.push({
      label: 'Sync',
      icon: <RotateCw className={syncing ? 'animate-spin' : undefined} />,
      disabled: syncing,
      onSelect: async () => {
        setSyncing(true)
        await alertOnFailure(onSync, `Failed to sync "${name}"`)
        setSyncing(false)
      },
    })
  }
  actions.push(...extraActions)
  if (onMarkWatched) {
    actions.push({
      label: 'Mark all watched',
      icon: <CheckCheck />,
      disabled: markingWatched,
      onSelect: async () => {
        if (markWatchedPrompt) {
          setConfirmingMarkWatched(true)
          return
        }
        setMarkingWatched(true)
        await alertOnFailure(onMarkWatched, `Failed to mark "${name}" watched`)
        setMarkingWatched(false)
      },
    })
  }
  if (excludedFromHome !== undefined && onSetExcludedFromHome) {
    actions.push({
      label: excludedFromHome ? 'Include in home' : 'Exclude from home',
      icon: excludedFromHome ? <Eye /> : <EyeOff />,
      onSelect: () =>
        alertOnFailure(
          () => onSetExcludedFromHome(!excludedFromHome),
          `Failed to ${homeSettingAction(!excludedFromHome, name)}`,
        ),
    })
  }
  if (unwatchedCountHidden !== undefined && onSetUnwatchedCountHidden) {
    actions.push({
      label: unwatchedCountHidden ? 'Show unwatched count' : 'Hide unwatched count',
      icon: unwatchedCountHidden ? <Bell /> : <BellOff />,
      onSelect: () =>
        alertOnFailure(
          () => onSetUnwatchedCountHidden(!unwatchedCountHidden),
          `Failed to ${unwatchedCountSettingAction(!unwatchedCountHidden, name)}`,
        ),
    })
  }
  if (onEditRequest) {
    actions.push({ label: 'Edit settings', icon: <SlidersHorizontal />, onSelect: onEditRequest })
  }
  const requestDelete = () => setConfirmingDelete(true)

  return (
    <header
      className={cn(
        'mb-4 flex items-center gap-3 md:mb-6',
        tint && '-mx-2 rounded-xl px-2 py-3 md:-mx-3 md:px-3',
      )}
      style={tint ? { background: tint } : undefined}
    >
      {colorControl}
      {showAvatar && (
        <Thumbnail
          src={avatarSrc}
          className="size-10 shrink-0 rounded-full object-cover md:size-12"
        />
      )}
      <div className="flex min-w-0 flex-1 flex-col">
        {onRename ? (
          <EditableTitle name={name} onRename={onRename} />
        ) : (
          <h2 className="truncate font-heading text-lg font-semibold text-foreground md:text-xl">
            {name}
          </h2>
        )}
        {summary && <p className="truncate text-xs text-muted-foreground md:text-sm">{summary}</p>}
      </div>
      {isDesktop ? (
        <TooltipProvider>
          <div className="flex shrink-0 items-center gap-1.5">
            {actions.map((action) => (
              <IconAction key={action.label} action={action} />
            ))}
            <MoreActionsMenu name={name} actions={[]} onDelete={requestDelete} desktop />
          </div>
        </TooltipProvider>
      ) : (
        <MoreActionsMenu name={name} actions={actions} onDelete={requestDelete} />
      )}

      <ConfirmDialog
        open={confirmingDelete}
        onOpenChange={setConfirmingDelete}
        title={`Delete "${name}"?`}
        description={deleteDescription}
        onConfirm={onDelete}
      />
      {onMarkWatched && markWatchedPrompt && (
        <ConfirmDialog
          open={confirmingMarkWatched}
          onOpenChange={setConfirmingMarkWatched}
          title={markWatchedPrompt.title}
          description={markWatchedPrompt.description}
          confirmLabel="Mark watched"
          pendingLabel="Marking…"
          onConfirm={onMarkWatched}
        />
      )}
    </header>
  )
}
