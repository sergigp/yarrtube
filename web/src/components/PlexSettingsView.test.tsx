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

  it("shows the setup card when Plex isn't configured", async () => {
    mockApi({
      'GET /api/plex': aPlexStatus({ configured: false, connection: null, syncs: [] }),
      'GET /api/preferences': aPreferences(),
    })

    renderWithProviders(<PlexSettingsView />)

    const setup = await screen.findByRole('group', { name: 'Watch your downloads in Plex' })
    expect(lines(setup)).toEqual([
      'Connect yarrtube to your Plex server and every channel and playlist shows up as a TV show.',
      'New episodes appear in Plex right after they download.',
      'Watched status stays in sync between yarrtube and Plex.',
      'Each category becomes a smart playlist.',
      'Set these environment variables and restart the container',
    ])
    expect(within(setup).getAllByRole('listitem').map((row) => row.textContent)).toEqual([
      "YARRTUBE_PLEX_URL · your server's addressCopy",
      'YARRTUBE_PLEX_TOKEN · your Plex tokenCopy',
      'YARRTUBE_PLEX_VIDEOS_PATH · where Plex sees the videos folderCopy',
    ])
    expect(within(setup).getByRole('link', { name: 'Open the Plex guide' })).toHaveAttribute(
      'href',
      'https://github.com/sergigp/yarrtube/blob/main/doc/PLEX.md',
    )
    expect(screen.queryByRole('button', { name: /Sync now/ })).not.toBeInTheDocument()
    expect(screen.queryByRole('group', { name: 'Category playlists' })).not.toBeInTheDocument()
  })

  it('copies a variable name', async () => {
    mockApi({
      'GET /api/plex': aPlexStatus({ configured: false, connection: null, syncs: [] }),
      'GET /api/preferences': aPreferences(),
    })
    renderWithProviders(<PlexSettingsView />)
    const user = userEvent.setup()

    const copy = await screen.findByRole('button', { name: 'Copy YARRTUBE_PLEX_TOKEN' })
    await user.click(copy)

    expect(await navigator.clipboard.readText()).toBe('YARRTUBE_PLEX_TOKEN')
    expect(copy).toHaveTextContent('Copied')
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

  it('tells what to check when Plex is unreachable', async () => {
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
    expect(within(server).getByText("Can't reach Plex", { selector: 'span' })).toBeInTheDocument()
    expect(lines(server)).toEqual([
      "Can't reach Plex: connection refused. Check YARRTUBE_PLEX_URL and that the server is running.",
    ])
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

  it('shows Retry now on a failed sync', async () => {
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
            last_run_at: '2026-11-14T09:59:00Z',
            last_error: 'timed out',
            next_run_at: '2026-11-14T10:14:00Z',
          }),
        ],
      }),
      'GET /api/preferences': aPreferences(),
    })

    renderWithProviders(<PlexSettingsView />)

    const sync = await screen.findByRole('group', { name: 'Sync' })
    expect(sync).not.toHaveTextContent('Runs on its own')
    const watched = within(sync).getByRole('group', { name: 'Watched status' })
    const labels = within(sync).getByRole('group', { name: 'Categories to Plex' })
    expect(lines(watched)).toEqual([
      "Both ways: watch a video here or in Plex and it's marked watched in both.",
      'Last run 3 minutes ago · Next in 2 minutes',
    ])
    expect(lines(labels)).toEqual([
      'Category labels and playlists in Plex follow your categories.',
      'Failed 4 minutes ago: timed out. Retries in 11 minutes.',
    ])
    expect(within(watched).getByRole('button', { name: 'Sync now' })).toBeEnabled()
    expect(within(labels).getByRole('button', { name: 'Retry now' })).toBeEnabled()
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
    const watchState = await screen.findByRole('group', { name: 'Watched status' })

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
    const labels = await screen.findByRole('group', { name: 'Categories to Plex' })

    await user.click(within(labels).getByRole('button', { name: 'Sync now' }))

    expect(await within(labels).findByRole('alert')).toHaveTextContent(
      "Couldn't start the sync: database is locked",
    )
    expect(within(labels).getByRole('button', { name: 'Sync now' })).toBeEnabled()
  })

  it('saves a playlist choice and updates the preview', async () => {
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
    const preview = within(playlists).getByRole('figure', { name: 'Preview in Plex' })
    expect(within(watched).getByRole('radio', { name: 'Keep them' })).toBeChecked()
    expect(within(order).getByRole('radio', { name: 'Newest first' })).toBeChecked()
    expect(playlists).not.toHaveTextContent('queue of what')
    expect(previewRowTexts(preview)).toEqual([
      'Published today',
      'Published last week · Watched',
      'Published last month',
    ])

    await user.click(within(watched).getByRole('radio', { name: 'Remove them' }))

    await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('Saved'))
    expect(within(watched).getByRole('radio', { name: 'Remove them' })).toBeChecked()
    expect(
      fetchMock.mock.calls
        .filter(([, init]) => (init as RequestInit | undefined)?.method === 'PATCH')
        .map(([, init]) => JSON.parse(String((init as RequestInit).body))),
    ).toEqual([{ plex_playlist_watched: 'drop' }])
    expect(previewRowTexts(preview)).toEqual(['Published today', 'Published last month'])
    expect(preview).toHaveTextContent('Watched episodes are left out.')
    expect(playlists).toHaveTextContent(
      'Each category becomes a smart playlist in Plex with all its episodes. These options apply to every category playlist.',
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
      "Couldn't save: database is locked. Try again.",
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
    expect(playlists).toHaveTextContent('Category playlists need categories, which are off.')
  })

  it('turns categories on from the Plex page', async () => {
    let categoriesEnabled = false
    const fetchMock = mockApi({
      'GET /api/plex': aPlexStatus(),
      'GET /api/preferences': () => aPreferences({ categories_enabled: categoriesEnabled }),
      'PATCH /api/preferences': () => {
        categoriesEnabled = true
        return aPreferences({ categories_enabled: true })
      },
    })
    renderWithProviders(<PlexSettingsView />)
    const user = userEvent.setup()

    await user.click(await screen.findByRole('button', { name: 'Turn on categories' }))

    const playlists = screen.getByRole('group', { name: 'Category playlists' })
    await waitFor(() =>
      expect(
        within(playlists)
          .getAllByRole('radio')
          .every((radio) => !(radio as HTMLInputElement).disabled),
      ).toBe(true),
    )
    expect(playlists).not.toHaveTextContent('Category playlists need categories')
    expect(
      fetchMock.mock.calls
        .filter(([, init]) => (init as RequestInit | undefined)?.method === 'PATCH')
        .map(([, init]) => JSON.parse(String((init as RequestInit).body))),
    ).toEqual([{ categories_enabled: true }])
  })
})

/** The text of each example episode in the playlist `preview`. */
function previewRowTexts(preview: HTMLElement): string[] {
  return within(preview)
    .getAllByRole('listitem')
    .map((row) => row.textContent ?? '')
}

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
