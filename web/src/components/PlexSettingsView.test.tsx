import { afterEach, describe, expect, it, vi } from 'vitest'
import { screen, within } from '@testing-library/react'
import { PlexSettingsView } from './PlexSettingsView'
import {
  aPlexConnection,
  aPlexStatus,
  aPlexSync,
  aPreferences,
  mockApi,
  renderWithProviders,
} from '@/test/helpers'

describe('PlexSettingsView', () => {
  afterEach(() => {
    vi.useRealTimers()
  })

  it('explains how to set Plex up when it is not configured', async () => {
    mockApi({
      'GET /api/plex': aPlexStatus({ configured: false, connection: null, syncs: [] }),
      'GET /api/preferences': aPreferences(),
    })

    renderWithProviders(<PlexSettingsView />)

    expect(await screen.findByText(/Plex isn't configured/)).toHaveTextContent(
      "Plex isn't configured. Set YARRTUBE_PLEX_URL, YARRTUBE_PLEX_TOKEN and YARRTUBE_PLEX_VIDEOS_PATH to connect yarrtube to your Plex server.",
    )
    expect(screen.getByRole('link', { name: 'Plex guide' })).toHaveAttribute(
      'href',
      'https://github.com/sergigp/yarrtube/blob/main/doc/PLEX.md',
    )
    expect(screen.queryByRole('button', { name: /Sync now/ })).not.toBeInTheDocument()
    expect(screen.queryByRole('group', { name: 'Category playlists' })).not.toBeInTheDocument()
  })

  it('shows the connection the latest sync saw', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true })
    vi.setSystemTime(new Date('2026-11-14T10:02:00Z'))
    mockApi({
      'GET /api/plex': aPlexStatus({
        connection: aPlexConnection({
          checked_at: '2026-11-14T10:00:00Z',
          version: '1.43.5.11079-b3b13e506',
          libraries: ['Subscriptions', "Sergi's Playlists"],
        }),
      }),
      'GET /api/preferences': aPreferences(),
    })

    renderWithProviders(<PlexSettingsView />)

    const connection = await screen.findByRole('group', { name: 'Connection' })
    expect(lines(connection)).toEqual([
      'Connected',
      'Plex Media Server 1.43.5.11079',
      "Libraries: Subscriptions, Sergi's Playlists",
      'Checked 2 minutes ago',
    ])
  })

  it('says Plex cannot be reached, with the error and what was seen last', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true })
    vi.setSystemTime(new Date('2026-11-14T10:05:00Z'))
    mockApi({
      'GET /api/plex': aPlexStatus({
        connection: aPlexConnection({
          checked_at: '2026-11-14T10:04:00Z',
          reachable: false,
          error: 'connection refused',
          version: '1.43.5.11079-b3b13e506',
          libraries: ['Subscriptions'],
        }),
      }),
      'GET /api/preferences': aPreferences(),
    })

    renderWithProviders(<PlexSettingsView />)

    const connection = await screen.findByRole('group', { name: 'Connection' })
    expect(lines(connection)).toEqual([
      "Can't reach Plex: connection refused",
      'Plex Media Server 1.43.5.11079',
      'Libraries: Subscriptions',
      'Checked 1 minute ago',
    ])
  })

  it('says the status shows after the first sync', async () => {
    mockApi({
      'GET /api/plex': aPlexStatus({ connection: null }),
      'GET /api/preferences': aPreferences(),
    })

    renderWithProviders(<PlexSettingsView />)

    const connection = await screen.findByRole('group', { name: 'Connection' })
    expect(lines(connection)).toEqual(['The connection shows here after the first Plex sync.'])
  })

  it('shows when each sync last ran, how it ended and when it runs next', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true })
    vi.setSystemTime(new Date('2026-11-14T10:03:00Z'))
    mockApi({
      'GET /api/plex': aPlexStatus({
        syncs: [
          aPlexSync({
            kind: 'watch_state',
            last_run_at: '2026-11-14T10:00:00Z',
            next_run_at: '2026-11-14T10:05:00Z',
          }),
          aPlexSync({
            kind: 'labels',
            last_run_at: '2026-11-14T09:58:00Z',
            last_error: 'timed out',
            next_run_at: '2026-11-14T10:13:00Z',
          }),
        ],
      }),
      'GET /api/preferences': aPreferences(),
    })

    renderWithProviders(<PlexSettingsView />)

    const watchState = await screen.findByRole('group', { name: 'Watch state' })
    const labels = screen.getByRole('group', { name: 'Labels & playlists' })
    expect(lines(watchState)).toEqual(['Last run 3 minutes ago', 'Next run in 2 minutes'])
    expect(lines(labels)).toEqual([
      'Last run 5 minutes ago: timed out',
      'Next run in 10 minutes',
    ])
    expect(within(watchState).getByRole('button', { name: 'Sync now' })).toBeEnabled()
  })
})

/** The text of each line (paragraph) in `element`. */
function lines(element: HTMLElement): string[] {
  return Array.from(element.querySelectorAll('p')).map((line) => line.textContent ?? '')
}
