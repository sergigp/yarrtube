import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  byCategory,
  describeTask,
  matchesTask,
  tabCounts,
  tasksForTab,
  taskCategory,
  taskFamily,
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
  it('maps each task type to its family', () => {
    expect(taskFamily(aTask({ task_type: 'download_video' }))).toBe('downloads')
    expect(taskFamily(aTask({ task_type: 'fetch_thumbnail' }))).toBe('downloads')
    expect(taskFamily(aTask({ task_type: 'reconcile_playlist' }))).toBe('syncs')
    expect(taskFamily(aTask({ task_type: 'reconcile_channel' }))).toBe('syncs')
    expect(taskFamily(aTask({ task_type: 'reconcile_plex_collections' }))).toBe('syncs')
    expect(taskFamily(aTask({ task_type: 'delete_video_file' }))).toBe('cleanup')
    expect(taskFamily(aTask({ task_type: 'delete_playlist_files' }))).toBe('cleanup')
    expect(taskFamily(aTask({ task_type: 'delete_channel_files' }))).toBe('cleanup')
    expect(taskFamily(aTask({ task_type: 'update_ytdlp' }))).toBe('maintenance')
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

  it('returns the right types for the downloads, syncs and cleanup tabs', () => {
    const download = aTask({ id: 1, task_type: 'download_video', status: 'pending' })
    const thumbnail = aTask({ id: 2, task_type: 'fetch_thumbnail', status: 'pending' })
    const sync = aTask({ id: 3, task_type: 'reconcile_plex_collections', status: 'pending' })
    const cleanup = aTask({ id: 4, task_type: 'delete_channel_files', status: 'pending' })
    const all = [download, thumbnail, sync, cleanup]

    expect(tasksForTab(all, 'downloads').map((task) => task.id).sort()).toEqual([1, 2])
    expect(tasksForTab(all, 'syncs').map((task) => task.id)).toEqual([3])
    expect(tasksForTab(all, 'cleanup').map((task) => task.id)).toEqual([4])
  })

  it("returns every task sorted by category for the 'all' tab", () => {
    const pendingLater = aTask({
      id: 1,
      task_type: 'download_video',
      status: 'pending',
      run_at: '2999-01-01T00:00:00Z',
    })
    const running = aTask({ id: 2, task_type: 'reconcile_channel', status: 'running' })
    const pendingSoon = aTask({
      id: 3,
      task_type: 'delete_video_file',
      status: 'pending',
      run_at: '2100-01-01T00:00:00Z',
    })

    const all = tasksForTab([pendingLater, running, pendingSoon], 'all')

    expect(all.map((task) => task.id)).toEqual([2, 3, 1])
  })

  it("places a running maintenance task in 'all' and 'active' but no family tab", () => {
    const maintenance = aTask({ id: 1, task_type: 'update_ytdlp', status: 'running' })
    const tasks = [maintenance]

    expect(tasksForTab(tasks, 'all').map((task) => task.id)).toEqual([1])
    expect(tasksForTab(tasks, 'active').map((task) => task.id)).toEqual([1])
    expect(tasksForTab(tasks, 'downloads')).toEqual([])
    expect(tasksForTab(tasks, 'syncs')).toEqual([])
    expect(tasksForTab(tasks, 'cleanup')).toEqual([])
  })

  it("returns counts equal to each tab's listed length", () => {
    const tasks = [
      aTask({ id: 1, task_type: 'download_video', status: 'running' }),
      aTask({ id: 2, task_type: 'fetch_thumbnail', status: 'pending' }),
      aTask({ id: 3, task_type: 'reconcile_channel', status: 'pending' }),
      aTask({ id: 4, task_type: 'delete_video_file', status: 'pending' }),
      aTask({ id: 5, task_type: 'update_ytdlp', status: 'pending' }),
    ]

    const counts = tabCounts(tasks)

    for (const tab of ['active', 'downloads', 'syncs', 'cleanup', 'all'] as const) {
      expect(counts[tab]).toBe(tasksForTab(tasks, tab).length)
    }
    expect(counts).toEqual({ active: 1, downloads: 2, syncs: 1, cleanup: 1, all: 5 })
  })
})
