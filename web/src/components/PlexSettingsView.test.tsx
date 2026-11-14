import { afterEach, describe, expect, it, vi } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { PlexSettingsView } from './PlexSettingsView'
import {
  aPlexConnection,
  aPlexStatus,
  aPlexSync,
  aPreferences,
  heldUntilReleased,
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

  it('shows the server badge, version, libraries and last checked', async () => {
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

    const server = await screen.findByRole('group', { name: 'Server' })
    expect(within(server).getByText('Connected')).toBeInTheDocument()
    expect(details(server)).toEqual([
      ['Version', '1.43.5.11079'],
      ['Libraries', "Subscriptions, Sergi's Playlists"],
      ['Last checked', '2 minutes ago'],
    ])
    expect(
      screen.getByText(
        'Keep watched status in sync with Plex and turn categories into Plex playlists.',
      ),
    ).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'Plex guide' })).toHaveAttribute(
      'href',
      'https://github.com/sergigp/yarrtube/blob/main/doc/PLEX.md',
    )
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

    const server = await screen.findByRole('group', { name: 'Server' })
    expect(lines(server)).toEqual(["Can't reach Plex: connection refused"])
    expect(details(server)).toEqual([
      ['Version', '1.43.5.11079'],
      ['Libraries', 'Subscriptions'],
      ['Last checked', '1 minute ago'],
    ])
  })

  it('says the server is not checked yet before the first sync', async () => {
    mockApi({
      'GET /api/plex': aPlexStatus({ connection: null }),
      'GET /api/preferences': aPreferences(),
    })

    renderWithProviders(<PlexSettingsView />)

    const server = await screen.findByRole('group', { name: 'Server' })
    expect(lines(server)).toEqual(['Not checked yet. Server details appear after the first sync.'])
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

  it('runs a sync now and shows it syncing until a newer run finishes', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true })
    vi.setSystemTime(new Date('2026-11-14T10:00:00Z'))
    let lastRunAt = '2026-11-14T09:58:00Z'
    const post = heldUntilReleased()
    const fetchMock = mockApi({
      'GET /api/plex': () =>
        aPlexStatus({ syncs: [aPlexSync({ kind: 'watch_state', last_run_at: lastRunAt })] }),
      'POST /api/plex/syncs/watch_state': post.route,
      'GET /api/preferences': aPreferences(),
    })
    renderWithProviders(<PlexSettingsView />)
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime })
    const watchState = await screen.findByRole('group', { name: 'Watch state' })

    await user.click(within(watchState).getByRole('button', { name: 'Sync now' }))

    expect(within(watchState).getByRole('button', { name: 'Syncing…' })).toBeDisabled()
    await waitFor(() => expect(post.route).toHaveBeenCalled())
    lastRunAt = '2026-11-14T10:00:30Z'
    post.release()
    await waitFor(() =>
      expect(within(watchState).getByRole('button', { name: 'Sync now' })).toBeEnabled(),
    )
    expect(fetchMock).toHaveBeenCalledWith('/api/plex/syncs/watch_state', {
      method: 'POST',
    })
  })

  it('shows why a sync could not be requested and lets the user try again', async () => {
    mockApi({
      'GET /api/plex': aPlexStatus({ syncs: [aPlexSync({ kind: 'labels' })] }),
      'POST /api/plex/syncs/labels': { status: 500, error: 'database is locked' },
      'GET /api/preferences': aPreferences(),
    })
    renderWithProviders(<PlexSettingsView />)
    const user = userEvent.setup()
    const labels = await screen.findByRole('group', { name: 'Labels & playlists' })

    await user.click(within(labels).getByRole('button', { name: 'Sync now' }))

    expect(await within(labels).findByRole('alert')).toHaveTextContent(
      "Couldn't start the sync: database is locked",
    )
    expect(within(labels).getByRole('button', { name: 'Sync now' })).toBeEnabled()
  })

  it('saves a category playlist option on its own', async () => {
    const fetchMock = mockApi({
      'GET /api/plex': aPlexStatus(),
      'GET /api/preferences': aPreferences(),
      'PATCH /api/preferences': aPreferences({ plex_playlist_watched: 'drop' }),
    })
    renderWithProviders(<PlexSettingsView />)
    const user = userEvent.setup()
    const playlists = await screen.findByRole('group', { name: 'Category playlists' })
    const watched = within(playlists).getByRole('radiogroup', { name: 'Watched episodes' })
    const order = within(playlists).getByRole('radiogroup', { name: 'Order' })
    expect(within(watched).getByRole('radio', { name: 'Stay in the playlist' })).toBeChecked()
    expect(within(order).getByRole('radio', { name: 'Newest first' })).toBeChecked()

    await user.click(within(watched).getByRole('radio', { name: 'Drop out' }))

    await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('Saved'))
    expect(within(watched).getByRole('radio', { name: 'Drop out' })).toBeChecked()
    expect(
      fetchMock.mock.calls
        .filter(([, init]) => (init as RequestInit | undefined)?.method === 'PATCH')
        .map(([, init]) => JSON.parse(String((init as RequestInit).body))),
    ).toEqual([{ plex_playlist_watched: 'drop' }])
    expect(playlists).toHaveTextContent(
      'With categories on, each category becomes a smart playlist in Plex with all its episodes. These options apply to every category playlist.',
    )
  })

  it('shows why a playlist option was not saved and keeps the saved one', async () => {
    mockApi({
      'GET /api/plex': aPlexStatus(),
      'GET /api/preferences': aPreferences(),
      'PATCH /api/preferences': { status: 500, error: 'database is locked' },
    })
    renderWithProviders(<PlexSettingsView />)
    const user = userEvent.setup()
    const order = await screen.findByRole('radiogroup', { name: 'Order' })

    await user.click(within(order).getByRole('radio', { name: 'Oldest first' }))

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Failed to save: database is locked',
    )
    expect(within(order).getByRole('radio', { name: 'Newest first' })).toBeChecked()
    expect(screen.getByRole('status')).toHaveTextContent('')
  })

  it('disables the playlist options while categories are off', async () => {
    mockApi({
      'GET /api/plex': aPlexStatus(),
      'GET /api/preferences': aPreferences({ categories_enabled: false }),
    })

    renderWithProviders(<PlexSettingsView />)

    const playlists = await screen.findByRole('group', { name: 'Category playlists' })
    expect(
      within(playlists)
        .getAllByRole('radio')
        .map((radio) => (radio as HTMLInputElement).disabled),
    ).toEqual([true, true, true, true])
    expect(playlists).toHaveTextContent(
      'Category playlists need categories on, in General settings.',
    )
  })
})

/** Each term of the description list in `element`, with its description. */
function details(element: HTMLElement): [string, string][] {
  const terms = within(element).getAllByRole('term')
  const definitions = within(element).getAllByRole('definition')
  return terms.map((term, index) => [term.textContent ?? '', definitions[index]?.textContent ?? ''])
}

/** The text of each line (paragraph) in `element`. */
function lines(element: HTMLElement): string[] {
  return Array.from(element.querySelectorAll('p')).map((line) => line.textContent ?? '')
}
