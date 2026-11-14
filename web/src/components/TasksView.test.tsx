import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { TasksView } from './TasksView'
import { aTask, mockApi, pendingForever, renderWithProviders } from '@/test/helpers'

const NOW = new Date('2026-11-14T20:03:00Z')

/** In `minutes` from `NOW`, as an ISO timestamp. */
function inMinutes(minutes: number): string {
  return new Date(NOW.getTime() + minutes * 60 * 1000).toISOString()
}

/** The clock time the browser shows for `iso`; its locale isn't pinned, only its time zone. */
function clockAt(iso: string): string {
  return new Intl.DateTimeFormat(undefined, { hour: '2-digit', minute: '2-digit' }).format(new Date(iso))
}

/** Like prod: a failing download, a few tasks within the hour and a batch of syncs later. */
function aProdLikeQueue() {
  return [
    aTask({
      id: 1,
      task_type: 'download_video',
      run_at: inMinutes(5),
      last_error: 'ERROR: unable to download video data: HTTP Error 403: Forbidden',
      payload: { video_id: 'rec1', video_title: 'My Apps', channel_name: 'Starter Story Build' },
    }),
    aTask({ id: 2, task_type: 'sync_plex_watch_state', run_at: inMinutes(2) }),
    aTask({ id: 3, task_type: 'sync_plex_labels', run_at: inMinutes(30) }),
    aTask({ id: 4, task_type: 'update_ytdlp', run_at: inMinutes(45) }),
    aTask({
      id: 5,
      task_type: 'fetch_thumbnail',
      run_at: inMinutes(50),
      payload: { video_id: 'rec2', video_title: 'Intro', channel_name: 'Chan' },
    }),
    aTask({
      id: 6,
      task_type: 'reconcile_channel',
      run_at: inMinutes(137),
      payload: { channel_id: '@3b1b', channel_name: '3Blue1Brown' },
    }),
    aTask({
      id: 7,
      task_type: 'reconcile_playlist',
      run_at: inMinutes(138),
      payload: { playlist_id: 'PL1', playlist_name: 'chess' },
    }),
  ]
}

/** The JSON bodies `fetchMock` sent to `key`, e.g. `"POST /api/tasks/run-now"`. */
function bodiesSentTo(fetchMock: ReturnType<typeof mockApi>, key: string): unknown[] {
  return fetchMock.mock.calls
    .filter(([url, init]) => `${init?.method ?? 'GET'} ${url}` === key)
    .map(([, init]) => JSON.parse(String(init?.body)))
}

/** Five channel and playlist checks due a minute apart, from 20:20. */
function aBatchOfSyncs() {
  return ['3Blue1Brown', 'Eric Rosen', 'chess', 'Jobava fans', 'Openings'].map((name, index) =>
    index % 2 === 0
      ? aTask({
          id: index + 1,
          task_type: 'reconcile_channel',
          run_at: inMinutes(17 + index),
          payload: { channel_id: `@${index}`, channel_name: name },
        })
      : aTask({
          id: index + 1,
          task_type: 'reconcile_playlist',
          run_at: inMinutes(17 + index),
          payload: { playlist_id: `PL${index}`, playlist_name: name },
        }),
  )
}

describe('TasksView', () => {
  beforeEach(() => {
    vi.useFakeTimers({ toFake: ['Date'] })
    vi.setSystemTime(NOW)
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('shows a loading message until the tasks arrive', () => {
    mockApi({ 'GET /api/tasks': pendingForever() })

    renderWithProviders(<TasksView />)

    expect(screen.getByText('Loading tasks…')).toBeInTheDocument()
  })

  it('shows the failure when the tasks cannot be loaded', async () => {
    mockApi({ 'GET /api/tasks': { status: 500, error: 'db locked' } })

    renderWithProviders(<TasksView />)

    expect(await screen.findByText('Failed to load tasks: db locked')).toBeInTheDocument()
  })

  it('opens on All with the status line and the sections, without status badges', async () => {
    mockApi({ 'GET /api/tasks': aProdLikeQueue() })

    renderWithProviders(<TasksView />)

    expect(
      await screen.findByText('Nothing running. Next up in 2 min: Sync watch state with Plex.'),
    ).toBeInTheDocument()
    expect(screen.getAllByRole('heading').map((heading) => heading.textContent)).toEqual([
      'Tasks',
      'Needs attention',
      'Next hour',
      'Later',
    ])
    expect(screen.getByText('Sync watch state with Plex')).toBeInTheDocument()
    expect(screen.getByText(clockAt(inMinutes(2)))).toBeInTheDocument()
    expect(screen.getByText('in 2 min')).toBeInTheDocument()
    expect(screen.queryByText(/^(pending|queued|running)$/)).not.toBeInTheDocument()
  })

  it('filters by chip, hiding chips with no tasks and marking the selected one pressed', async () => {
    const user = userEvent.setup()
    mockApi({ 'GET /api/tasks': aProdLikeQueue() })
    renderWithProviders(<TasksView />)

    const chips = await screen.findAllByRole('button', { pressed: false })
    expect(
      [screen.getByRole('button', { pressed: true }), ...chips].map((chip) => chip.textContent),
    ).toEqual(['All7', 'Downloads2', 'Syncs2', 'Plex & upkeep3'])

    await user.click(screen.getByRole('button', { name: /^Downloads/ }))

    expect(screen.getByRole('button', { name: /^Downloads/ })).toHaveAttribute('aria-pressed', 'true')
    expect(screen.getByRole('button', { name: /^All/ })).toHaveAttribute('aria-pressed', 'false')
    expect(screen.getByText('Fetch the thumbnail of Intro in Chan')).toBeInTheDocument()
    expect(screen.queryByText('Sync watch state with Plex')).not.toBeInTheDocument()
  })

  it('expands and collapses a batch of syncs, and syncs all of them now', async () => {
    const user = userEvent.setup()
    const fetchMock = mockApi({
      'GET /api/tasks': aBatchOfSyncs(),
      'POST /api/tasks/run-now': null,
    })
    renderWithProviders(<TasksView />)

    const batch = (
      await screen.findByText('Check 5 channels and playlists for new videos')
    ).closest('li')!
    expect(within(batch).getByText('3Blue1Brown, Eric Rosen, chess and 2 more')).toBeInTheDocument()

    await user.click(within(batch).getByRole('button', { name: 'Show sources' }))
    expect(
      within(batch)
        .getAllByRole('listitem')
        .map((item) => item.textContent),
    ).toEqual([
      expect.stringContaining('3Blue1BrownChannel'),
      expect.stringContaining('Eric RosenPlaylist'),
      expect.stringContaining('chessChannel'),
      expect.stringContaining('Jobava fansPlaylist'),
      expect.stringContaining('OpeningsChannel'),
    ])

    await user.click(within(batch).getByRole('button', { name: 'Hide sources' }))
    expect(within(batch).queryAllByRole('listitem')).toEqual([])

    await user.click(within(batch).getByRole('button', { name: 'Sync all now' }))
    await waitFor(() =>
      expect(bodiesSentTo(fetchMock, 'POST /api/tasks/run-now')).toEqual([{ ids: [1, 2, 3, 4, 5] }]),
    )
  })

  it('syncs one source of an open batch now', async () => {
    const user = userEvent.setup()
    const fetchMock = mockApi({
      'GET /api/tasks': aBatchOfSyncs(),
      'POST /api/tasks/run-now': null,
    })
    renderWithProviders(<TasksView />)

    await user.click(await screen.findByRole('button', { name: 'Show sources' }))
    const chess = screen.getByText('chess', { selector: 'li span' }).closest('li')!
    await user.click(within(chess).getByRole('button', { name: 'Sync now' }))

    await waitFor(() =>
      expect(bodiesSentTo(fetchMock, 'POST /api/tasks/run-now')).toEqual([{ ids: [3] }]),
    )
  })

  it('runs a pending Plex sync now, and offers no Run now on per-video or running tasks', async () => {
    const user = userEvent.setup()
    const fetchMock = mockApi({
      'GET /api/tasks': [
        ...aProdLikeQueue(),
        aTask({ id: 8, task_type: 'update_ytdlp', status: 'running', started_at: NOW.toISOString() }),
      ],
      'POST /api/tasks/run-now': null,
    })
    renderWithProviders(<TasksView />)

    const rowOf = (text: string) => screen.getByText(text).closest('li')!
    const plex = (await screen.findByText('Sync watch state with Plex')).closest('li')!
    expect([
      within(rowOf('Fetch the thumbnail of Intro in Chan')).queryByRole('button', { name: 'Run now' }),
      within(rowOf('Updating yt-dlp')).queryByRole('button', { name: 'Run now' }),
    ]).toEqual([null, null])

    await user.click(within(plex).getByRole('button', { name: 'Run now' }))

    await waitFor(() =>
      expect(bodiesSentTo(fetchMock, 'POST /api/tasks/run-now')).toEqual([{ ids: [2] }]),
    )
  })
})
