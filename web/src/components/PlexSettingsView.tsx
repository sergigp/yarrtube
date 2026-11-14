import type { ReactNode } from 'react'
import type { PlexConnection } from '@/api/types'
import { usePlexStatus } from '@/api/queries'
import { plexVersionLabel, relativeTime } from '@/lib/plexStatus'
import { cn } from '@/lib/utils'
import { VIEW_WIDTH } from '@/lib/viewWidth'

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
          {status.connection && <ConnectionStatus connection={status.connection} />}
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
      <p className="font-medium text-foreground">Connected</p>
      {connection.version && (
        <p className="text-muted-foreground">Plex Media Server {plexVersionLabel(connection.version)}</p>
      )}
      <p className="text-muted-foreground">Libraries: {connection.libraries.join(', ')}</p>
      <p className="text-muted-foreground">Checked {relativeTime(connection.checked_at)}</p>
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
