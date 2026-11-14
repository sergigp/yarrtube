import { useState, type ReactNode } from 'react'
import type { PlexConnection, PlexSync, PlexSyncKind } from '@/api/types'
import { usePlexStatus, useRequestPlexSync } from '@/api/queries'
import { isSyncing, plexVersionLabel, relativeTime } from '@/lib/plexStatus'
import { Button } from '@/components/ui/button'
import { errorMessage } from '@/lib/errorMessage'
import { cn } from '@/lib/utils'
import { VIEW_WIDTH } from '@/lib/viewWidth'

const SYNC_LABELS: Record<PlexSyncKind, string> = {
  watch_state: 'Watch state',
  labels: 'Labels & playlists',
}

const PLEX_GUIDE_URL = 'https://github.com/sergigp/yarrtube/blob/main/doc/PLEX.md'

/** The Plex section of the Settings page. */
export function PlexSettingsView() {
  const { data: status, error } = usePlexStatus()

  return (
    <div className={cn(VIEW_WIDTH, 'flex flex-col gap-8')}>
      <h1 className="font-heading text-xl font-semibold text-foreground">Plex</h1>
      {error && (
        <p className="text-sm text-destructive">Failed to load the Plex status: {error.message}</p>
      )}
      {status && !status.configured && <NotConfigured />}
      {status?.configured && (
        <Section title="Connection">
          {status.connection ? (
            <ConnectionStatus connection={status.connection} />
          ) : (
            <p className="text-sm text-muted-foreground">
              The connection shows here after the first Plex sync.
            </p>
          )}
        </Section>
      )}
      {status?.configured && (
        <Section title="Syncs">
          {status.syncs.map((sync) => (
            <SyncRow key={sync.kind} sync={sync} />
          ))}
        </Section>
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

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <fieldset className="flex flex-col gap-4">
      <legend className="mb-4 font-heading text-lg font-semibold text-foreground">{title}</legend>
      {children}
    </fieldset>
  )
}
