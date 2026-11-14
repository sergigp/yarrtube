import { SettingsCard } from '@/components/SettingsCard'
import { PageHeader } from '@/components/PageHeader'
import { Segmented } from '@/components/Segmented'
import { useState } from 'react'
import type {
  PlexConnection,
  PlexPlaylistOrder,
  PlexPlaylistWatched,
  PlexSync,
  PlexSyncKind,
  Preferences,
  PreferencesPatch,
} from '@/api/types'
import {
  usePlexStatus,
  usePreferences,
  useRequestPlexSync,
  useSavePreferences,
} from '@/api/queries'
import { useSavedFlash } from '@/hooks/useSavedFlash'
import { isSyncing, plexVersionLabel, relativeTime, syncStatusLine } from '@/lib/plexStatus'
import { Button } from '@/components/ui/button'
import { couldntSave, errorMessage } from '@/lib/errorMessage'
import { previewRows } from '@/lib/playlistPreview'
import { cn } from '@/lib/utils'
import { VIEW_WIDTH } from '@/lib/viewWidth'

const SYNCS: Record<PlexSyncKind, { label: string; description: string }> = {
  watch_state: {
    label: 'Watched status',
    description: "Both ways: watch a video here or in Plex and it's marked watched in both.",
  },
  labels: {
    label: 'Categories to Plex',
    description: 'Category labels and playlists in Plex follow your categories.',
  },
}

const WATCHED_CHOICES: { value: PlexPlaylistWatched; label: string }[] = [
  { value: 'keep', label: 'Keep them' },
  { value: 'drop', label: 'Remove them' },
]

const ORDER_CHOICES: { value: PlexPlaylistOrder; label: string }[] = [
  { value: 'newest_first', label: 'Newest first' },
  { value: 'oldest_first', label: 'Oldest first' },
]

const PLEX_GUIDE_URL = 'https://github.com/sergigp/yarrtube/blob/main/doc/PLEX.md'

/** The Plex section of the Settings page. */
export function PlexSettingsView() {
  const { data: status, error } = usePlexStatus()
  const { data: preferences } = usePreferences()
  const savePreferences = useSavePreferences()
  const saved = useSavedFlash()
  const [saveError, setSaveError] = useState<string | null>(null)
  const save = async (patch: PreferencesPatch) => {
    setSaveError(null)
    saved.clear()
    try {
      await savePreferences(patch)
      saved.flash()
    } catch (err) {
      setSaveError(errorMessage(err))
    }
  }

  return (
    <div className={cn(VIEW_WIDTH, 'flex flex-col gap-8')}>
      <PageHeader
        title="Plex"
        description={
          status?.configured && (
            <>
              Keep watched status in sync with Plex and turn categories into Plex playlists.{' '}
              <a
                href={PLEX_GUIDE_URL}
                target="_blank"
                rel="noreferrer"
                className="text-primary underline-offset-4 hover:underline"
              >
                Plex guide
              </a>
            </>
          )
        }
        saved={saved.shown}
      />
      {error && (
        <p className="text-sm text-destructive">Failed to load the Plex status: {error.message}</p>
      )}
      {status && !status.configured && <NotConfigured />}
      {status?.configured && (
        <SettingsCard
          title="Server"
          action={status.connection && <ConnectionBadge connection={status.connection} />}
        >
          {status.connection ? (
            <ConnectionStatus connection={status.connection} />
          ) : (
            <p className="text-sm text-muted-foreground">
              Not checked yet. Server details appear after the first sync.
            </p>
          )}
        </SettingsCard>
      )}
      {status?.configured && (
        <SettingsCard
          title="Sync"
          description={<p>Runs on its own. Use Sync now after changing something in Plex.</p>}
        >
          <div className="flex flex-col divide-y divide-border">
            {status.syncs.map((sync) => (
              <SyncRow key={sync.kind} sync={sync} />
            ))}
          </div>
        </SettingsCard>
      )}
      {status?.configured && preferences && (
        <SettingsCard
          title="Category playlists"
          description={
            <p>
              Each category becomes a smart playlist in Plex with all its episodes. These options
              apply to every category playlist.
            </p>
          }
        >
          {saveError && (
            <p role="alert" className="text-sm text-destructive">
              {couldntSave(saveError)}
            </p>
          )}
          <PlaylistOptions preferences={preferences} onSave={(patch) => void save(patch)} />
        </SettingsCard>
      )}
    </div>
  )
}

function NotConfigured() {
  return (
    <div className="flex flex-col gap-2 text-sm text-muted-foreground">
      <p>
        Plex isn&apos;t configured. Set <code>YARRTUBE_PLEX_URL</code>,{' '}
        <code>YARRTUBE_PLEX_TOKEN</code> and <code>YARRTUBE_PLEX_VIDEOS_PATH</code> to connect
        yarrtube to your Plex server.
      </p>
      <p>
        See the{' '}
        <a
          href={PLEX_GUIDE_URL}
          target="_blank"
          rel="noreferrer"
          className="underline hover:text-foreground"
        >
          Plex guide
        </a>{' '}
        for every step.
      </p>
    </div>
  )
}

/** "Connected" or "Can't reach Plex", as a badge. */
function ConnectionBadge({ connection }: { connection: PlexConnection }) {
  return (
    <span
      className={cn(
        'inline-flex shrink-0 items-center gap-1.5 rounded-full px-3 py-1 text-sm font-semibold',
        connection.reachable
          ? 'bg-accent text-primary'
          : 'bg-destructive/10 text-destructive',
      )}
    >
      <span
        aria-hidden="true"
        className={cn(
          'size-1.5 rounded-full',
          connection.reachable ? 'bg-primary' : 'bg-destructive',
        )}
      />
      {connection.reachable ? 'Connected' : "Can't reach Plex"}
    </span>
  )
}

/** What the latest sync saw of the server: version, libraries, and when. */
function ConnectionStatus({ connection }: { connection: PlexConnection }) {
  return (
    <div className="flex flex-col gap-3 text-sm">
      {!connection.reachable && (
        <p className="text-destructive">
          Can&apos;t reach Plex: {connection.error}. Check YARRTUBE_PLEX_URL and that the server
          is running.
        </p>
      )}
      <dl className="grid grid-cols-1 gap-4 sm:grid-cols-3">
        {connection.version && (
          <div>
            <dt className="text-xs text-muted-foreground">Version</dt>
            <dd className="mt-0.5">{plexVersionLabel(connection.version)}</dd>
          </div>
        )}
        <div>
          <dt className="text-xs text-muted-foreground">Libraries</dt>
          <dd className="mt-0.5">{connection.libraries.join(', ')}</dd>
        </div>
        <div>
          <dt className="text-xs text-muted-foreground">Last checked</dt>
          <dd className="mt-0.5">{relativeTime(connection.checked_at)}</dd>
        </div>
      </dl>
    </div>
  )
}

function SyncRow({ sync }: { sync: PlexSync }) {
  const { label, description } = SYNCS[sync.kind]
  const requestSync = useRequestPlexSync()
  const [requestedAt, setRequestedAt] = useState<Date | null>(null)
  const [requestError, setRequestError] = useState<string | null>(null)
  const syncing = isSyncing(sync, requestedAt)

  async function syncNow() {
    setRequestError(null)
    setRequestedAt(new Date())
    try {
      await requestSync(sync.kind)
    } catch (e) {
      setRequestedAt(null)
      setRequestError(errorMessage(e))
    }
  }

  return (
    <div
      role="group"
      aria-label={label}
      className="flex items-center justify-between gap-4 py-3.5 text-sm first:pt-0 last:pb-0"
    >
      <div className="flex flex-col gap-0.5">
        <span className="text-[15px] font-medium text-foreground" aria-hidden="true">
          {label}
        </span>
        <p className="text-muted-foreground">{description}</p>
        <p className={sync.last_error ? 'text-destructive' : 'text-muted-foreground'}>
          {syncStatusLine(sync)}
        </p>
        {requestError && (
          <p role="alert" className="text-destructive">
            Couldn&apos;t start the sync: {requestError}
          </p>
        )}
      </div>
      <Button variant="outline" size="sm" disabled={syncing} onClick={() => void syncNow()}>
        {syncing ? 'Syncing…' : sync.last_error ? 'Retry now' : 'Sync now'}
      </Button>
    </div>
  )
}

function PlaylistOptions({
  preferences,
  onSave,
}: {
  preferences: Preferences
  onSave: (patch: PreferencesPatch) => void
}) {
  const disabled = !preferences.categories_enabled
  return (
    <div className="flex flex-col gap-4 text-sm">
      {disabled && (
        <p className="text-muted-foreground">
          Category playlists need categories on, in General settings.
        </p>
      )}
      <div className="flex flex-wrap gap-8">
        <div className="flex min-w-64 flex-1 flex-col gap-5">
          <div className="flex flex-col gap-2">
            <span className="text-[15px] font-medium text-foreground" aria-hidden="true">
              Watched episodes
            </span>
            <Segmented
              name="Watched episodes"
              choices={WATCHED_CHOICES}
              value={preferences.plex_playlist_watched}
              disabled={disabled}
              onChange={(value) => onSave({ plex_playlist_watched: value })}
            />
            <p className="text-muted-foreground">
              Removing them turns each playlist into a queue of what&apos;s left to watch.
            </p>
          </div>
          <div className="flex flex-col gap-2">
            <span className="text-[15px] font-medium text-foreground" aria-hidden="true">
              Order
            </span>
            <Segmented
              name="Order"
              choices={ORDER_CHOICES}
              value={preferences.plex_playlist_order}
              disabled={disabled}
              onChange={(value) => onSave({ plex_playlist_order: value })}
            />
          </div>
        </div>
        <PlaylistPreview
          watched={preferences.plex_playlist_watched}
          order={preferences.plex_playlist_order}
        />
      </div>
    </div>
  )
}

/** Example episodes of a category playlist, as Plex lists them with the options chosen. */
function PlaylistPreview({
  watched,
  order,
}: {
  watched: PlexPlaylistWatched
  order: PlexPlaylistOrder
}) {
  return (
    <figure
      aria-label="Preview in Plex"
      className="m-0 flex min-w-60 flex-1 flex-col gap-1.5 rounded-lg border border-border p-3.5"
    >
      <figcaption className="mb-1 text-xs text-muted-foreground" aria-hidden="true">
        Preview in Plex
      </figcaption>
      <ul className="flex flex-col gap-1.5">
        {previewRows(watched, order).map((row) => (
          <li
            key={row.label}
            className="flex items-center gap-2.5 rounded-md bg-muted/60 px-2.5 py-2 text-sm"
          >
            <span aria-hidden="true" className="h-7 w-12 shrink-0 rounded bg-accent" />
            <span className={row.watched ? 'text-muted-foreground' : 'text-foreground'}>
              {row.label}
              {row.watched && ' · Watched'}
            </span>
          </li>
        ))}
      </ul>
      {watched === 'drop' && (
        <p className="mt-1 text-xs text-muted-foreground">Watched episodes are left out.</p>
      )}
    </figure>
  )
}
