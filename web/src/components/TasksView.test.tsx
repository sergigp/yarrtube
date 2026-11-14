import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { screen } from '@testing-library/react'
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
})
