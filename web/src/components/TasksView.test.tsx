import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { TasksView } from './TasksView'
import {
  aTask,
  heldUntilReleased,
  mockApi,
  pendingForever,
  renderWithProviders,
} from '@/test/helpers'

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

  it('explains a failing task in plain words, showing the raw error on demand', async () => {
    const user = userEvent.setup()
    const raw = 'ERROR: unable to download video data: HTTP Error 403: Forbidden'
    mockApi({ 'GET /api/tasks': aProdLikeQueue() })
    renderWithProviders(<TasksView />)

    const card = (
      await screen.findByText('Download My Apps from Starter Story Build')
    ).closest('li')!
    expect(
      within(card).getByText(
        `YouTube refused the request (403). Yarrtube tries again at ${clockAt(inMinutes(5))}, in 5 min.`,
      ),
    ).toBeInTheDocument()
    expect(within(card).queryByText(raw)).not.toBeInTheDocument()

    await user.click(within(card).getByRole('button', { name: 'Show error' }))
    expect(within(card).getByText(raw)).toBeInTheDocument()

    await user.click(within(card).getByRole('button', { name: 'Hide error' }))
    expect(within(card).queryByText(raw)).not.toBeInTheDocument()
  })

  it('retries the video now, disabling Retry now while it asks, then refreshes the list, offering no Stop retrying', async () => {
    const user = userEvent.setup()
    const retry = heldUntilReleased()
    const fetchMock = mockApi({
      'GET /api/tasks': aProdLikeQueue(),
      'POST /api/videos/rec1/retry': retry.route,
    })
    renderWithProviders(<TasksView />)
    const card = (await screen.findByText('Download My Apps from Starter Story Build')).closest('li')!

    await user.click(within(card).getByRole('button', { name: 'Retry now' }))

    await waitFor(() => expect(retry.route).toHaveBeenCalledOnce())
    expect(within(card).getByRole('button', { name: 'Retry now' })).toBeDisabled()
    expect(within(card).queryByRole('button', { name: 'Stop retrying' })).not.toBeInTheDocument()
    retry.release()
    await waitFor(() =>
      expect(fetchMock.mock.calls.filter(([url]) => url === '/api/tasks')).toHaveLength(2),
    )
    expect(within(card).getByRole('button', { name: 'Retry now' })).toBeEnabled()
  })

  it('says when retrying fails, keeping Retry now usable', async () => {
    const user = userEvent.setup()
    mockApi({
      'GET /api/tasks': aProdLikeQueue(),
      'POST /api/videos/rec1/retry': { status: 500, error: 'db locked' },
    })
    renderWithProviders(<TasksView />)
    const card = (await screen.findByText('Download My Apps from Starter Story Build')).closest('li')!

    await user.click(within(card).getByRole('button', { name: 'Retry now' }))

    expect(await within(card).findByText('Retry failed: db locked')).toBeInTheDocument()
    expect(within(card).getByRole('button', { name: 'Retry now' })).toBeEnabled()
  })

  it('discards a failing download after confirming', async () => {
    const user = userEvent.setup()
    const [download, ...rest] = aProdLikeQueue()
    let queue = [download, ...rest]
    const discard = vi.fn(() => {
      queue = rest
      return null
    })
    mockApi({
      'GET /api/tasks': () => queue,
      'POST /api/videos/rec1/discard': discard,
    })
    renderWithProviders(<TasksView />)
    const card = (await screen.findByText('Download My Apps from Starter Story Build')).closest('li')!

    await user.click(within(card).getByRole('button', { name: 'Discard' }))
    const dialog = await screen.findByRole('dialog', { name: 'Discard "My Apps"?' })
    await user.click(within(dialog).getByRole('button', { name: 'Discard' }))

    expect(discard).toHaveBeenCalledOnce()
    await waitFor(() =>
      expect(screen.queryByText('Download My Apps from Starter Story Build')).not.toBeInTheDocument(),
    )
  })

  it('says the discard failed', async () => {
    const user = userEvent.setup()
    mockApi({
      'GET /api/tasks': aProdLikeQueue(),
      'POST /api/videos/rec1/discard': { status: 500, error: 'db locked' },
    })
    renderWithProviders(<TasksView />)
    const card = (await screen.findByText('Download My Apps from Starter Story Build')).closest('li')!

    await user.click(within(card).getByRole('button', { name: 'Discard' }))
    const dialog = await screen.findByRole('dialog', { name: 'Discard "My Apps"?' })
    await user.click(within(dialog).getByRole('button', { name: 'Discard' }))

    expect(await within(card).findByText('Discard failed: db locked')).toBeInTheDocument()
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
    expect(within(card).getByRole('button', { name: 'Discard' })).toBeEnabled()
  })

  it('says when running a task now fails, and lets it be tried again', async () => {
    const user = userEvent.setup()
    const fetchMock = mockApi({
      'GET /api/tasks': aProdLikeQueue(),
      'POST /api/tasks/run-now': { status: 500, error: 'db locked' },
    })
    renderWithProviders(<TasksView />)
    const plex = (await screen.findByText('Sync watch state with Plex')).closest('li')!

    await user.click(within(plex).getByRole('button', { name: 'Run now' }))
    expect(await within(plex).findByText('Run now failed: db locked')).toBeInTheDocument()
    await user.click(within(plex).getByRole('button', { name: 'Run now' }))

    await waitFor(() => expect(bodiesSentTo(fetchMock, 'POST /api/tasks/run-now')).toHaveLength(2))
  })

  it('runs a failing task that works on no video now from its card', async () => {
    const user = userEvent.setup()
    const fetchMock = mockApi({
      'GET /api/tasks': [
        aTask({ id: 2, task_type: 'sync_plex_watch_state', run_at: inMinutes(2), last_error: 'Plex unreachable' }),
      ],
      'POST /api/tasks/run-now': null,
    })
    renderWithProviders(<TasksView />)
    const attention = await screen.findByRole('region', { name: 'Needs attention' })

    await user.click(within(attention).getByRole('button', { name: 'Run now' }))

    await waitFor(() =>
      expect(bodiesSentTo(fetchMock, 'POST /api/tasks/run-now')).toEqual([{ ids: [2] }]),
    )
  })

  it('goes back to All when the selected chip has no tasks left, staying there when its tasks return', async () => {
    const user = userEvent.setup()
    const [download, plex] = aProdLikeQueue()
    const queues = [[download, plex], [plex], [download, plex]]
    mockApi({
      'GET /api/tasks': () => queues.shift() ?? [download, plex],
      'POST /api/videos/rec1/retry': null,
      'POST /api/tasks/run-now': null,
    })
    renderWithProviders(<TasksView />)
    await user.click(await screen.findByRole('button', { name: /^Downloads/ }))

    await user.click(screen.getByRole('button', { name: 'Retry now' }))
    expect(await screen.findByText('Sync watch state with Plex')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Run now' }))

    expect(await screen.findByText('Download My Apps from Starter Story Build')).toBeInTheDocument()
    expect([
      screen.getByRole('button', { name: /^All/ }).getAttribute('aria-pressed'),
      screen.queryByText('Sync watch state with Plex') !== null,
    ]).toEqual(['true', true])
  })

  it('describes every task button by its task, so screen readers tell them apart', async () => {
    const user = userEvent.setup()
    const syncs = aBatchOfSyncs().map((task) => ({ ...task, id: task.id + 10 }))
    mockApi({ 'GET /api/tasks': [...aProdLikeQueue().slice(0, 4), ...syncs] })
    renderWithProviders(<TasksView />)
    await user.click(await screen.findByRole('button', { name: 'Show sources' }))

    /** The text each button named `name` is described by, in order. */
    const describing = (name: string) =>
      screen
        .getAllByRole('button', { name })
        .map((button) => document.getElementById(button.getAttribute('aria-describedby') ?? '')?.textContent)
    expect({
      retry: describing('Retry now'),
      error: describing('Show error'),
      runNow: describing('Run now'),
      syncAll: describing('Sync all now'),
      sources: describing('Hide sources'),
      syncNow: describing('Sync now'),
    }).toEqual({
      retry: ['Download My Apps from Starter Story Build'],
      error: ['Download My Apps from Starter Story Build'],
      runNow: ['Sync watch state with Plex', 'Update category labels in Plex', 'Update yt-dlp'],
      syncAll: ['Check 5 channels and playlists for new videos'],
      sources: ['Check 5 channels and playlists for new videos'],
      syncNow: ['3Blue1Brown', 'Eric Rosen', 'chess', 'Jobava fans', 'Openings'],
    })
  })

  it('searches within the selected chip, keeping the text when the chip changes', async () => {
    const user = userEvent.setup()
    mockApi({ 'GET /api/tasks': [...aProdLikeQueue(), ...aBatchOfSyncs().map((task) => ({ ...task, id: task.id + 10 }))] })
    renderWithProviders(<TasksView />)

    const search = await screen.findByRole('searchbox', { name: 'Search tasks' })
    await user.type(search, 'jobava')
    expect(screen.getByText('Check Jobava fans for new videos')).toBeInTheDocument()
    expect(screen.queryByText('Sync watch state with Plex')).not.toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: /^Downloads/ }))
    expect(search).toHaveValue('jobava')
    expect(screen.getByText('Nothing matches your search.')).toBeInTheDocument()

    await user.clear(search)
    expect(screen.getByText('Fetch the thumbnail of Intro in Chan')).toBeInTheDocument()
  })

  it('says all is quiet, with no chips, when nothing is scheduled', async () => {
    mockApi({ 'GET /api/tasks': [] })
    renderWithProviders(<TasksView />)

    expect(await screen.findByText('All quiet. Nothing is scheduled.')).toBeInTheDocument()
    expect(screen.getByRole('heading', { name: 'Tasks' })).toBeInTheDocument()
    expect(screen.queryByRole('button')).not.toBeInTheDocument()
  })

  it('keeps chip names on a phone in a sideways-scrolling row, with 44 px controls and stacked rows', async () => {
    mockApi({ 'GET /api/tasks': aProdLikeQueue() })
    renderWithProviders(<TasksView />)

    const chips = await screen.findAllByRole('button', { name: /^(All|Downloads|Syncs|Plex & upkeep)/ })
    const card = screen.getByText('Download My Apps from Starter Story Build').closest('li')!
    const plex = screen.getByText('Sync watch state with Plex').closest('li')!
    const controls = [
      ...chips,
      within(card).getByRole('button', { name: 'Retry now' }),
      within(plex).getByRole('button', { name: 'Run now' }),
    ]

    expect(chips.map((chip) => chip.querySelector('.sr-only'))).toEqual(chips.map(() => null))
    expect(chips[0].parentElement).toHaveClass('overflow-x-auto')
    expect(controls.filter((control) => !control.classList.contains('min-h-11'))).toEqual([])
    expect(plex).toHaveClass('flex-col', 'sm:flex-row')
  })
})
