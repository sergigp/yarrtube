import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  byCategory,
  describeTask,
  matchesTask,
  tabCounts,
  tasksForTab,
  taskCategory,
  taskFamily,
  taskKind,
} from './tasks'
import { aTask } from '@/test/helpers'

describe('describeTask', () => {
  it('describes syncing tasks by their target name', () => {
    expect(
      describeTask(aTask({ task_type: 'reconcile_playlist', payload: { playlist_name: 'Mix' } })),
    ).toBe('Syncing playlist Mix')
    expect(
      describeTask(aTask({ task_type: 'reconcile_channel', payload: { channel_name: 'Chan' } })),
    ).toBe('Syncing channel Chan')
  })

  it('falls back to generic names when the payload is missing details', () => {
    expect(describeTask(aTask({ task_type: 'reconcile_playlist', payload: {} }))).toBe(
      'Syncing playlist an unknown playlist',
    )
    expect(describeTask(aTask({ task_type: 'download_video', payload: {} }))).toBe(
      'Downloading a video in an unknown playlist or channel',
    )
  })

  it('describes a download with its video and container', () => {
    expect(
      describeTask(
        aTask({
          task_type: 'download_video',
          payload: { video_title: 'Intro', channel_name: 'Chan' },
        }),
      ),
    ).toBe('Downloading Intro in Chan')
  })

  it('describes a thumbnail fetch with its video and container', () => {
    expect(
      describeTask(
        aTask({
          task_type: 'fetch_thumbnail',
          payload: { video_title: 'Intro', playlist_name: 'Mix' },
        }),
      ),
    ).toBe('Fetching thumbnail of Intro in Mix')
    expect(describeTask(aTask({ task_type: 'fetch_thumbnail', payload: {} }))).toBe(
      'Fetching thumbnail of a video in an unknown playlist or channel',
    )
  })

  it('describes file removals', () => {
    expect(
      describeTask(aTask({ task_type: 'delete_video_file', payload: { filename: 'a.mp4' } })),
    ).toBe('Removing file a.mp4')
    expect(describeTask(aTask({ task_type: 'delete_video_file', payload: {} }))).toBe(
      'Removing a video file',
    )
  })

  it('describes a Plex collection reconciliation in plain language', () => {
    expect(describeTask(aTask({ task_type: 'reconcile_plex_collections', payload: {} }))).toBe(
      'Syncing Plex collections',
    )
  })

  it('describes a yt-dlp self-update in plain language', () => {
    expect(describeTask(aTask({ task_type: 'update_ytdlp', payload: {} }))).toBe('Updating yt-dlp')
  })

  it('humanizes an unknown type instead of showing the raw snake_case string', () => {
    const description = describeTask(aTask({ task_type: 'mystery_task', payload: {} }))
    expect(description).not.toContain('_')
    expect(description).toBe('Mystery task')
  })

  it('describes path deletions and yt-dlp updates for other task types', () => {
    expect(describeTask(aTask({ task_type: 'delete_files', payload: { path: 'playlists/x' } }))).toBe(
      'Deleting files at playlists/x',
    )
    expect(describeTask(aTask({ task_type: 'update_ytdlp', payload: {} }))).toBe('Updating yt-dlp')
    expect(describeTask(aTask({ task_type: 'mystery_task', payload: {} }))).toBe('Mystery task')
  })
})

describe('taskCategory and byCategory', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-06-15T12:00:00Z'))
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('passes a non-pending status through as the category', () => {
    expect(taskCategory(aTask({ status: 'running' }))).toBe('running')
  })

  it('splits pending tasks into queued (due) and pending (scheduled later)', () => {
    expect(taskCategory(aTask({ status: 'pending', run_at: '2026-06-15T11:00:00Z' }))).toBe(
      'queued',
    )
    expect(taskCategory(aTask({ status: 'pending', run_at: '2026-06-15T13:00:00Z' }))).toBe(
      'pending',
    )
  })

  it('orders running before queued before pending, then by run time', () => {
    const pendingLater = aTask({ status: 'pending', run_at: '2026-06-15T15:00:00Z' })
    const pendingSoon = aTask({ status: 'pending', run_at: '2026-06-15T13:00:00Z' })
    const queued = aTask({ status: 'pending', run_at: '2026-06-15T11:00:00Z' })
    const running = aTask({ status: 'running', run_at: '2026-06-15T10:00:00Z' })

    const sorted = [pendingLater, queued, pendingSoon, running].sort(byCategory)

    expect(sorted).toEqual([running, queued, pendingSoon, pendingLater])
  })
})

describe('taskFamily, tasksForTab, tabCounts and matchesTask', () => {
  it('maps each task type to downloads, syncs or other, and unknown types to other', () => {
    expect(taskFamily(aTask({ task_type: 'download_video' }))).toBe('downloads')
    expect(taskFamily(aTask({ task_type: 'fetch_thumbnail' }))).toBe('downloads')
    expect(taskFamily(aTask({ task_type: 'reconcile_playlist' }))).toBe('syncs')
    expect(taskFamily(aTask({ task_type: 'reconcile_channel' }))).toBe('syncs')
    expect(taskFamily(aTask({ task_type: 'reconcile_plex_collections' }))).toBe('other')
    expect(taskFamily(aTask({ task_type: 'update_ytdlp' }))).toBe('other')
    expect(taskFamily(aTask({ task_type: 'delete_video_file' }))).toBe('other')
    expect(taskFamily(aTask({ task_type: 'delete_playlist_files' }))).toBe('other')
    expect(taskFamily(aTask({ task_type: 'delete_channel_files' }))).toBe('other')
    expect(taskFamily(aTask({ task_type: 'mystery_task' }))).toBe('other')
  })

  it("returns only running tasks of any type for the 'active' tab", () => {
    const runningSync = aTask({ id: 1, task_type: 'reconcile_channel', status: 'running' })
    const runningDownload = aTask({ id: 2, task_type: 'download_video', status: 'running' })
    const pendingDownload = aTask({
      id: 3,
      task_type: 'download_video',
      status: 'pending',
      run_at: '2999-01-01T00:00:00Z',
    })

    const active = tasksForTab([pendingDownload, runningSync, runningDownload], 'active')

    expect(active.map((task) => task.id).sort()).toEqual([1, 2])
    expect(active.every((task) => task.status === 'running')).toBe(true)
  })

  it('lists only playlist and channel reconciles in the syncs tab', () => {
    const tasks = [
      aTask({ id: 1, task_type: 'reconcile_playlist' }),
      aTask({ id: 2, task_type: 'reconcile_channel' }),
      aTask({ id: 3, task_type: 'reconcile_plex_collections' }),
      aTask({ id: 4, task_type: 'download_video' }),
      aTask({ id: 5, task_type: 'update_ytdlp' }),
    ]

    expect(tasksForTab(tasks, 'syncs').map((task) => task.id)).toEqual([1, 2])
  })

  it('collects Plex sync, yt-dlp update, deletions and unknown types in the other tab', () => {
    const tasks = [
      aTask({ id: 1, task_type: 'reconcile_plex_collections' }),
      aTask({ id: 2, task_type: 'update_ytdlp' }),
      aTask({ id: 3, task_type: 'delete_video_file' }),
      aTask({ id: 4, task_type: 'delete_playlist_files' }),
      aTask({ id: 5, task_type: 'delete_channel_files' }),
      aTask({ id: 6, task_type: 'mystery_task' }),
      aTask({ id: 7, task_type: 'download_video' }),
      aTask({ id: 8, task_type: 'reconcile_channel' }),
    ]

    expect(tasksForTab(tasks, 'other').map((task) => task.id)).toEqual([1, 2, 3, 4, 5, 6])
  })

  it('returns counts where downloads, syncs and other add up to every task', () => {
    const tasks = [
      aTask({ id: 1, task_type: 'download_video', status: 'running' }),
      aTask({ id: 2, task_type: 'fetch_thumbnail', status: 'pending' }),
      aTask({ id: 3, task_type: 'reconcile_channel', status: 'pending' }),
      aTask({ id: 4, task_type: 'reconcile_plex_collections', status: 'pending' }),
      aTask({ id: 5, task_type: 'delete_video_file', status: 'pending' }),
      aTask({ id: 6, task_type: 'update_ytdlp', status: 'running' }),
    ]

    expect(tabCounts(tasks)).toEqual({ active: 2, downloads: 2, syncs: 1, other: 3 })
  })

  it('maps each task type to its icon kind', () => {
    expect(taskKind(aTask({ task_type: 'download_video' }))).toBe('download')
    expect(taskKind(aTask({ task_type: 'fetch_thumbnail' }))).toBe('download')
    expect(taskKind(aTask({ task_type: 'reconcile_playlist' }))).toBe('sync')
    expect(taskKind(aTask({ task_type: 'reconcile_channel' }))).toBe('sync')
    expect(taskKind(aTask({ task_type: 'reconcile_plex_collections' }))).toBe('sync')
    expect(taskKind(aTask({ task_type: 'delete_video_file' }))).toBe('delete')
    expect(taskKind(aTask({ task_type: 'delete_playlist_files' }))).toBe('delete')
    expect(taskKind(aTask({ task_type: 'delete_channel_files' }))).toBe('delete')
    expect(taskKind(aTask({ task_type: 'update_ytdlp' }))).toBe('maintenance')
    expect(taskKind(aTask({ task_type: 'mystery_task' }))).toBe('maintenance')
  })

  it('matches on the description, ignoring case', () => {
    const task = aTask({ task_type: 'reconcile_channel', payload: { channel_name: 'Rustaceans' } })

    expect(matchesTask(task, 'rustaceans')).toBe(true)
    expect(matchesTask(task, 'SYNCING')).toBe(true)
    expect(matchesTask(task, 'nope')).toBe(false)
  })
})
