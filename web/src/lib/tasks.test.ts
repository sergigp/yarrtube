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
})
