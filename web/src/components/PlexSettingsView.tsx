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
import { isSyncing, plexVersionLabel, relativeTime } from '@/lib/plexStatus'
import { Button } from '@/components/ui/button'
import { errorMessage } from '@/lib/errorMessage'
import { cn } from '@/lib/utils'
import { VIEW_WIDTH } from '@/lib/viewWidth'

const SYNC_LABELS: Record<PlexSyncKind, string> = {
  watch_state: 'Watch state',
  labels: 'Labels & playlists',
}

const WATCHED_CHOICES: { value: PlexPlaylistWatched; label: string }[] = [
  { value: 'keep', label: 'Stay in the playlist' },
  { value: 'drop', label: 'Drop out' },
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
      <PageHeader title="Plex" saved={saved.shown} />
      {error && (
        <p className="text-sm text-destructive">Failed to load the Plex status: {error.message}</p>
      )}
      {status && !status.configured && <NotConfigured />}
      {status?.configured && (
        <SettingsCard title="Connection">
          {status.connection ? (
            <ConnectionStatus connection={status.connection} />
          ) : (
            <p className="text-sm text-muted-foreground">
              The connection shows here after the first Plex sync.
            </p>
          )}
        </SettingsCard>
      )}
      {status?.configured && (
        <SettingsCard title="Syncs">
          {status.syncs.map((sync) => (
            <SyncRow key={sync.kind} sync={sync} />
          ))}
        </SettingsCard>
      )}
      {status?.configured && preferences && (
        <SettingsCard title="Category playlists">
          {saveError && (
            <p role="alert" className="text-sm text-destructive">
              Failed to save: {saveError}
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

function ConnectionStatus({ connection }: { connection: PlexConnection }) {
  return (
    <div className="flex flex-col gap-1 text-sm">
      {connection.reachable ? (
        <p className="font-medium text-foreground">Connected</p>
      ) : (
        <p className="font-medium text-destructive">Can&apos;t reach Plex: {connection.error}</p>
      )}
      {connection.version && (
        <p className="text-muted-foreground">Plex Media Server {plexVersionLabel(connection.version)}</p>
      )}
      <p className="text-muted-foreground">Libraries: {connection.libraries.join(', ')}</p>
      <p className="text-muted-foreground">Checked {relativeTime(connection.checked_at)}</p>
    </div>
  )
}

function SyncRow({ sync }: { sync: PlexSync }) {
  const label = SYNC_LABELS[sync.kind]
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
      className="flex items-center justify-between gap-4 text-sm"
    >
      <div className="flex flex-col gap-1">
        <span className="font-medium text-foreground" aria-hidden="true">
          {label}
        </span>
        <p className={sync.last_error ? 'text-destructive' : 'text-muted-foreground'}>
          {sync.last_run_at ? `Last run ${relativeTime(sync.last_run_at)}` : 'Not run yet'}
          {sync.last_error && `: ${sync.last_error}`}
        </p>
        {sync.next_run_at && (
          <p className="text-muted-foreground">Next run {relativeTime(sync.next_run_at)}</p>
        )}
        {requestError && (
          <p role="alert" className="text-destructive">
            Couldn&apos;t start the sync: {requestError}
          </p>
        )}
      </div>
      <Button variant="outline" size="sm" disabled={syncing} onClick={() => void syncNow()}>
        {syncing ? 'Syncing…' : 'Sync now'}
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
      <p className="text-muted-foreground">
        With categories on, each category becomes a smart playlist in Plex with all its episodes.
        These options apply to every category playlist.
      </p>
      <Segmented
        name="Watched episodes"
        choices={WATCHED_CHOICES}
        value={preferences.plex_playlist_watched}
        disabled={disabled}
        onChange={(value) => onSave({ plex_playlist_watched: value })}
      />
      <Segmented
        name="Order"
        choices={ORDER_CHOICES}
        value={preferences.plex_playlist_order}
        disabled={disabled}
        onChange={(value) => onSave({ plex_playlist_order: value })}
      />
    </div>
  )
}
