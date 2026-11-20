import { useState, type ReactNode } from 'react'
import { Link } from 'react-router-dom'
import {
  Bell,
  BellOff,
  CheckCheck,
  Eraser,
  EllipsisVertical,
  ExternalLink,
  Eye,
  EyeOff,
  RotateCw,
  SlidersHorizontal,
  Sparkles,
  Trash2,
} from 'lucide-react'
import { ConfirmDialog } from './ConfirmDialog'
import { PlaylistIcon } from './PlaylistIcon'
import { Thumbnail } from './Thumbnail'
import { Button } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/skeleton'
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

/** One page-level action: an icon button on desktop, or an item of the "⋮" menu. */
export interface HeaderAction {
  label: string
  icon: ReactNode
  /** Runs on select; ignored when `href` is set. */
  onSelect?: () => void
  /** Makes the action a link opening `href` in a new tab instead. */
  href?: string
  disabled?: boolean
  /** On desktop, laid out as an icon button rather than offered in the "⋮" menu. */
  iconOnDesktop?: boolean
}

/** What stands for the source before its name. */
export type HeaderBadge =
  | { kind: 'avatar'; src: string | null }
  | { kind: 'playlist' }
  | { kind: 'color'; color: string }

/** Runs `action`, alerting `failure: <error>` when it throws. */
async function alertOnFailure(action: () => Promise<void>, failure: string): Promise<void> {
  try {
    await action()
  } catch (err) {
    window.alert(`${failure}: ${errorMessage(err)}`)
  }
}

/** A round, icon-only button naming its action in a tooltip and its accessible name. */
function IconAction({ action, compact }: { action: HeaderAction; compact: boolean }) {
  const className = cn('rounded-full', compact ? 'size-9' : 'size-10')
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        {action.href ? (
          <Button variant="outline" size="icon" className={className} asChild>
            <a href={action.href} target="_blank" rel="noopener" aria-label={action.label}>
              {action.icon}
            </a>
          </Button>
        ) : (
          <Button
            variant="outline"
            size="icon"
            className={className}
            aria-label={action.label}
            disabled={action.disabled}
            onClick={action.onSelect}
          >
            {action.icon}
          </Button>
        )}
      </TooltipTrigger>
      <TooltipContent>{action.label}</TooltipContent>
    </Tooltip>
  )
}

/**
 * The "⋮" menu: every action not laid out as an icon button, then delete,
 * set apart. Non-modal so opening a confirmation from it doesn't leave the
 * page with pointer events disabled. Labelled apart from the sidebar row's
 * "Actions for <name>", which is on screen at the same time.
 */
function MoreActionsMenu({
  name,
  actions,
  onDelete,
  className,
}: {
  name: string
  actions: HeaderAction[]
  onDelete: () => void
  className?: string
}) {
  return (
    <DropdownMenu modal={false}>
      <DropdownMenuTrigger asChild>
        <Button
          variant="outline"
          size="icon"
          className={cn('rounded-full', className)}
          aria-label={`More actions for ${name}`}
        >
          <EllipsisVertical />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-auto min-w-44">
        {actions.map((action) =>
          action.href ? (
            <DropdownMenuItem key={action.label} asChild>
              <a href={action.href} target="_blank" rel="noopener">
                {action.icon}
                {action.label}
              </a>
            </DropdownMenuItem>
          ) : (
            <DropdownMenuItem
              key={action.label}
              disabled={action.disabled}
              onSelect={action.onSelect}
            >
              {action.icon}
              {action.label}
            </DropdownMenuItem>
          ),
        )}
        {actions.length > 0 && <DropdownMenuSeparator />}
        <DropdownMenuItem variant="destructive" onSelect={onDelete}>
          <Trash2 />
          Delete
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

/** The square before the name: a channel's avatar, a playlist's icon or a category's colour. */
function Badge({ badge, compact }: { badge: HeaderBadge; compact: boolean }) {
  const size = compact ? 'size-10 rounded-[10px]' : 'size-13 rounded-xl md:size-16 md:rounded-[14px]'
  switch (badge.kind) {
    case 'avatar':
      return (
        <Thumbnail
          src={badge.src}
          className={cn('shrink-0 rounded-full! object-cover', size)}
        />
      )
    case 'playlist':
      return (
        <PlaylistIcon
          className={cn(
            size,
            '[&>svg]:size-5',
            !compact && 'md:[&>svg]:size-7 [&>svg]:size-6',
          )}
        />
      )
    case 'color':
      return (
        <span
          aria-hidden="true"
          className={cn('flex shrink-0 items-center justify-center bg-muted', size)}
        >
          <span
            className={cn('rounded-full', compact ? 'size-3' : 'size-4 md:size-4.5')}
            style={{ background: badge.color }}
          />
        </span>
      )
  }
}

interface DetailHeaderProps {
  name: string
  badge: HeaderBadge
  /** Makes the name a link, e.g. back to the playlist's page while watching it. */
  nameHref?: string
  nameLinkLabel?: string
  /** The meta line, e.g. "Playlist · 32 videos · 29 unwatched"; null holds its place while loading. */
  summary: string | null
  /** Shown after the summary, e.g. the source's categories. */
  chips?: ReactNode
  /** The main call to action, e.g. "Play next"; full width on phones. */
  primaryAction?: ReactNode
  /** Smaller, for the watching layout where the player is the point. */
  compact?: boolean
  /** Real count of unwatched videos, named by the mark-all-watched confirmation. */
  unwatchedCount?: number
  /** Shows the Sync action; categories have none. */
  onSync?: () => Promise<void>
  /** Actions listed right after Sync, e.g. a category's "Edit category". */
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
  /**
   * Shows a link to the source's YouTube page, labelled to name the source
   * (e.g. "Open channel on YouTube") apart from the selected video's link.
   */
  youtubeLink?: { href: string; label: string }
  /**
   * Channels and playlists, while categories are on: deletes the source's AI
   * verdicts, re-running its categorization when `enabled`, clearing its AI
   * categories otherwise.
   */
  aiCategorization?: { enabled: boolean; onDeleteAiVerdicts: () => Promise<void> }
  /** Channels and playlists: restores the source's discarded videos; categories have none. */
  onRestoreDiscarded?: () => Promise<void>
  onDelete: () => Promise<void>
  deleteDescription: string
}

/**
 * Title block at the top of a channel, playlist or category page: a badge,
 * the name, a meta line, then the main call to action and the page-level
 * actions. On desktop, Sync, the YouTube link and any extra action are icon
 * buttons named in their tooltips, the rest (mark all watched, home and
 * count settings, edit, AI, delete) wait behind "⋮"; on phones the header
 * stacks, the call to action goes full width and every action is behind "⋮".
 * Mark all watched and delete are always confirmed: neither can be undone.
 */
export function DetailHeader({
  name,
  badge,
  nameHref,
  nameLinkLabel,
  summary,
  chips,
  primaryAction,
  compact = false,
  unwatchedCount,
  onSync,
  extraActions = [],
  onMarkWatched,
  excludedFromHome,
  onSetExcludedFromHome,
  unwatchedCountHidden,
  onSetUnwatchedCountHidden,
  onEditRequest,
  youtubeLink,
  aiCategorization,
  onDelete,
  deleteDescription,
}: DetailHeaderProps) {
  const isDesktop = useIsDesktop()
  const [syncing, setSyncing] = useState(false)
  const [confirmingDelete, setConfirmingDelete] = useState(false)
  const [confirmingMarkWatched, setConfirmingMarkWatched] = useState(false)
  const [confirmingAiDelete, setConfirmingAiDelete] = useState(false)
  const markWatchedPrompt = markWatchedConfirmation(name, unwatchedCount ?? 0)

  const actions: HeaderAction[] = []
  if (onSync) {
    actions.push({
      label: 'Sync',
      icon: <RotateCw className={syncing ? 'animate-spin' : undefined} />,
      disabled: syncing,
      iconOnDesktop: true,
      onSelect: async () => {
        setSyncing(true)
        await alertOnFailure(onSync, `Failed to sync "${name}"`)
        setSyncing(false)
      },
    })
  }
  if (youtubeLink) {
    actions.push({
      label: youtubeLink.label,
      icon: <ExternalLink />,
      href: youtubeLink.href,
      iconOnDesktop: true,
    })
  }
  actions.push(...extraActions)
  if (onMarkWatched) {
    actions.push({
      label: 'Mark all watched',
      icon: <CheckCheck />,
      onSelect: () => setConfirmingMarkWatched(true),
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
  const aiDeleteLabel = aiCategorization?.enabled
    ? 'Re-run AI categorization'
    : 'Clear AI categories'
  if (aiCategorization) {
    actions.push({
      label: aiDeleteLabel,
      icon: aiCategorization.enabled ? <Sparkles /> : <Eraser />,
      onSelect: () => setConfirmingAiDelete(true),
    })
  }
  const requestDelete = () => setConfirmingDelete(true)
  const iconActions = isDesktop ? actions.filter((action) => action.iconOnDesktop) : []
  const menuActions = actions.filter((action) => !iconActions.includes(action))
  const menu = (
    <MoreActionsMenu
      name={name}
      actions={menuActions}
      onDelete={requestDelete}
      className={compact ? 'size-9' : 'size-11 md:size-10'}
    />
  )

  return (
    <header
      className={cn(
        'flex flex-col md:flex-row md:flex-wrap md:items-center',
        compact ? 'mb-4 gap-3 md:mb-5 md:gap-3.5' : 'mb-4 gap-3.5 md:mb-6 md:gap-5',
      )}
    >
      <div className={cn('flex min-w-0 items-center md:flex-1', compact ? 'gap-3' : 'gap-3.5 md:gap-5')}>
        <Badge badge={badge} compact={compact} />
        <div className={cn('flex min-w-0 flex-1 flex-col', compact ? 'gap-0.5' : 'gap-1 md:gap-1.5')}>
          <h2
            className={cn(
              'truncate font-heading font-bold tracking-tight text-foreground',
              compact ? 'text-lg md:text-[1.375rem]' : 'text-2xl md:text-3xl',
            )}
          >
            {nameHref ? (
              <Link
                to={nameHref}
                title={nameLinkLabel}
                className="no-underline transition-colors hover:text-primary"
              >
                {name}
              </Link>
            ) : (
              name
            )}
          </h2>
          <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 text-[0.8125rem] text-muted-foreground">
            {summary ? (
              <p className="min-w-0 md:truncate">{summary}</p>
            ) : (
              // Holds the summary's line until the videos it counts have loaded.
              <Skeleton className="my-0.5 h-3.5 w-40" />
            )}
            {chips}
          </div>
        </div>
        {!isDesktop && menu}
      </div>
      {(primaryAction || isDesktop) && (
        <div className="flex shrink-0 flex-wrap items-center gap-2">
          {primaryAction && <div className="max-md:w-full max-md:*:w-full">{primaryAction}</div>}
          {isDesktop && (
            <TooltipProvider>
              {iconActions.map((action) => (
                <IconAction key={action.label} action={action} compact={compact} />
              ))}
              {menu}
            </TooltipProvider>
          )}
        </div>
      )}

      <ConfirmDialog
        open={confirmingDelete}
        onOpenChange={setConfirmingDelete}
        title={`Delete "${name}"?`}
        description={deleteDescription}
        onConfirm={onDelete}
      />
      {aiCategorization && (
        <ConfirmDialog
          open={confirmingAiDelete}
          onOpenChange={setConfirmingAiDelete}
          title={`${aiDeleteLabel} of "${name}"?`}
          description={
            aiCategorization.enabled
              ? 'Its videos lose the categories the AI gave them and are evaluated again. Categories you set by hand stay.'
              : 'Its videos lose the categories the AI gave them. Categories you set by hand stay.'
          }
          confirmLabel={aiCategorization.enabled ? 'Re-run' : 'Clear'}
          pendingLabel={aiCategorization.enabled ? 'Re-running…' : 'Clearing…'}
          onConfirm={aiCategorization.onDeleteAiVerdicts}
        />
      )}
      {onMarkWatched && (
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
