import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  batchReconciles,
  buildTaskList,
  byCategory,
  describeTask,
  matchesTask,
  sectionOf,
  statusLine,
  syncTarget,
  tabCounts,
  tasksForTab,
  taskCategory,
  taskFamily,
  taskKind,
  taskTabs,
} from './tasks'
import { aTask } from '@/test/helpers'

describe('taskFamily', () => {
  it('puts downloads, fetches and quality checks under downloads', () => {
    const types = [
      'download_video',
      'fetch_thumbnail',
      'fetch_subtitles',
      'fetch_chapters',
      'upgrade_quality',
    ]

    expect(types.map((type) => taskFamily(aTask({ task_type: type }), false))).toEqual([
      'downloads',
      'downloads',
      'downloads',
      'downloads',
      'downloads',
    ])
  })

  it('puts playlist, channel and video syncs under syncs', () => {
    const types = ['reconcile_playlist', 'reconcile_channel', 'sync_video']

    expect(types.map((type) => taskFamily(aTask({ task_type: type }), false))).toEqual([
      'syncs',
      'syncs',
      'syncs',
    ])
  })

  it('puts summaries and categorizations under ai only while AI is on', () => {
    const types = ['summarize_video', 'categorize_video']

    expect(types.map((type) => taskFamily(aTask({ task_type: type }), true))).toEqual(['ai', 'ai'])
    expect(types.map((type) => taskFamily(aTask({ task_type: type }), false))).toEqual([
      'upkeep',
      'upkeep',
    ])
  })

  it('puts Plex syncs, yt-dlp updates, deletions and unknown types under Plex & upkeep', () => {
    const types = [
      'sync_plex_watch_state',
      'sync_plex_labels',
      'update_ytdlp',
      'delete_video_file',
      'delete_playlist_files',
      'delete_channel_files',
      'mystery_task',
    ]

    expect(types.map((type) => taskFamily(aTask({ task_type: type }), true))).toEqual(
      types.map(() => 'upkeep'),
    )
  })
})

describe('describeTask', () => {
  const video = { video_title: 'Intro', channel_name: 'Chan' }
  const cases: [type: string, payload: Record<string, string>, pending: string, running: string][] = [
    ['reconcile_channel', { channel_name: '3Blue1Brown' }, 'Check 3Blue1Brown for new videos', 'Checking 3Blue1Brown for new videos'],
    ['reconcile_playlist', { playlist_name: 'Mix' }, 'Check Mix for new videos', 'Checking Mix for new videos'],
    ['download_video', video, 'Download Intro from Chan', 'Downloading Intro from Chan'],
    ['fetch_thumbnail', video, 'Fetch the thumbnail of Intro in Chan', 'Fetching the thumbnail of Intro in Chan'],
    ['fetch_subtitles', video, 'Fetch the subtitles of Intro in Chan', 'Fetching the subtitles of Intro in Chan'],
    ['fetch_chapters', video, 'Fetch the chapters of Intro in Chan', 'Fetching the chapters of Intro in Chan'],
    ['upgrade_quality', video, 'Check the quality of Intro in Chan', 'Checking the quality of Intro in Chan'],
    ['summarize_video', video, 'Summarize Intro in Chan', 'Summarizing Intro in Chan'],
    ['categorize_video', video, 'Categorize Intro in Chan', 'Categorizing Intro in Chan'],
    ['sync_video', video, 'Sync Intro in Chan', 'Syncing Intro in Chan'],
    ['sync_plex_watch_state', {}, 'Sync watch state with Plex', 'Syncing watch state with Plex'],
    ['sync_plex_labels', {}, 'Update category labels in Plex', 'Updating category labels in Plex'],
    ['update_ytdlp', {}, 'Update yt-dlp', 'Updating yt-dlp'],
    ['delete_video_file', { filename: 'a.mp4' }, 'Remove the file a.mp4', 'Removing the file a.mp4'],
    ['delete_playlist_files', { path: 'playlists/x' }, 'Delete the files at playlists/x', 'Deleting the files at playlists/x'],
    ['delete_channel_files', { path: 'creators/x' }, 'Delete the files at creators/x', 'Deleting the files at creators/x'],
    ['mystery_task', {}, 'Mystery task', 'Mystery task'],
  ]

  it.each(cases)('describes a pending %s as what it will do', (type, payload, pending) => {
    expect(describeTask(aTask({ task_type: type, payload }))).toBe(pending)
  })

  it.each(cases)('describes a running %s as what it is doing', (type, payload, _pending, running) => {
    expect(describeTask(aTask({ task_type: type, payload, status: 'running' }))).toBe(running)
  })

  it('uses placeholders for names it could not resolve', () => {
    const types = ['reconcile_channel', 'reconcile_playlist', 'download_video', 'fetch_subtitles', 'delete_video_file']

    expect(types.map((type) => describeTask(aTask({ task_type: type, payload: {} })))).toEqual([
      'Check an unknown channel for new videos',
      'Check an unknown playlist for new videos',
      'Download a video from an unknown playlist or channel',
      'Fetch the subtitles of a video in an unknown playlist or channel',
      'Remove a video file',
    ])
  })

  it('never says pending or queued', () => {
    const descriptions = cases.map(([type, payload]) => describeTask(aTask({ task_type: type, payload })))

    expect(descriptions.filter((text) => /pending|queued/i.test(text))).toEqual([])
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


  it('labels a categorize_video task and files it under AI', () => {
    const task = aTask({
      task_type: 'categorize_video',
      payload: { video_title: 'Intro', channel_name: 'Chan' },
    })

    expect([describeTask(task), taskFamily(task, true), taskKind(task)]).toEqual([
      'Categorize Intro in Chan',
      'ai',
      'ai',
    ])
  })


  it('lists the ai tab between syncs and other only while AI is on', () => {
    expect(taskTabs(true)).toEqual(['active', 'downloads', 'syncs', 'ai', 'upkeep'])
    expect(taskTabs(false)).toEqual(['active', 'downloads', 'syncs', 'upkeep'])
  })

  it('counts every task once across tabs while AI is on', () => {
    const tasks = [
      aTask({ id: 1, task_type: 'summarize_video', status: 'running' }),
      aTask({ id: 2, task_type: 'summarize_video' }),
      aTask({ id: 3, task_type: 'download_video' }),
      aTask({ id: 4, task_type: 'reconcile_channel' }),
      aTask({ id: 5, task_type: 'update_ytdlp' }),
    ]

    expect(tabCounts(tasks, true)).toEqual({ active: 1, downloads: 1, syncs: 1, ai: 2, upkeep: 1 })
  })

  it('gives video summaries the ai kind', () => {
    expect(taskKind(aTask({ task_type: 'summarize_video' }))).toBe('ai')
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

    const active = tasksForTab([pendingDownload, runningSync, runningDownload], 'active', false)

    expect(active.map((task) => task.id).sort()).toEqual([1, 2])
    expect(active.every((task) => task.status === 'running')).toBe(true)
  })

  it('lists only playlist and channel reconciles in the syncs tab', () => {
    const tasks = [
      aTask({ id: 1, task_type: 'reconcile_playlist' }),
      aTask({ id: 2, task_type: 'reconcile_channel' }),
      aTask({ id: 3, task_type: 'download_video' }),
      aTask({ id: 4, task_type: 'update_ytdlp' }),
    ]

    expect(tasksForTab(tasks, 'syncs', false).map((task) => task.id)).toEqual([1, 2])
  })

  it('collects yt-dlp update, deletions and unknown types in the other tab', () => {
    const tasks = [
      aTask({ id: 1, task_type: 'update_ytdlp' }),
      aTask({ id: 2, task_type: 'delete_video_file' }),
      aTask({ id: 3, task_type: 'delete_playlist_files' }),
      aTask({ id: 4, task_type: 'delete_channel_files' }),
      aTask({ id: 5, task_type: 'mystery_task' }),
      aTask({ id: 6, task_type: 'download_video' }),
      aTask({ id: 7, task_type: 'reconcile_channel' }),
    ]

    expect(tasksForTab(tasks, 'upkeep', false).map((task) => task.id)).toEqual([1, 2, 3, 4, 5])
  })

  it('returns counts where downloads, syncs and other add up to every task', () => {
    const tasks = [
      aTask({ id: 1, task_type: 'download_video', status: 'running' }),
      aTask({ id: 2, task_type: 'fetch_thumbnail', status: 'pending' }),
      aTask({ id: 3, task_type: 'reconcile_channel', status: 'pending' }),
      aTask({ id: 4, task_type: 'delete_video_file', status: 'pending' }),
      aTask({ id: 5, task_type: 'update_ytdlp', status: 'running' }),
    ]

    expect(tabCounts(tasks, false)).toEqual({ active: 2, downloads: 2, syncs: 1, ai: 0, upkeep: 2 })
  })

  it('maps each task type to its icon kind', () => {
    expect(taskKind(aTask({ task_type: 'download_video' }))).toBe('download')
    expect(taskKind(aTask({ task_type: 'fetch_thumbnail' }))).toBe('download')
    expect(taskKind(aTask({ task_type: 'fetch_subtitles' }))).toBe('download')
    expect(taskKind(aTask({ task_type: 'reconcile_playlist' }))).toBe('sync')
    expect(taskKind(aTask({ task_type: 'reconcile_channel' }))).toBe('sync')
    expect(taskKind(aTask({ task_type: 'sync_video' }))).toBe('sync')
    expect(taskKind(aTask({ task_type: 'delete_video_file' }))).toBe('delete')
    expect(taskKind(aTask({ task_type: 'delete_playlist_files' }))).toBe('delete')
    expect(taskKind(aTask({ task_type: 'delete_channel_files' }))).toBe('delete')
    expect(taskKind(aTask({ task_type: 'update_ytdlp' }))).toBe('maintenance')
    expect(taskKind(aTask({ task_type: 'mystery_task' }))).toBe('maintenance')
  })

  it('returns a sync target for pending playlist and channel reconciles only', () => {
    expect(
      syncTarget(aTask({ task_type: 'reconcile_playlist', payload: { playlist_id: 'PL1' } })),
    ).toEqual({ kind: 'playlist', id: 'PL1' })
    expect(
      syncTarget(aTask({ task_type: 'reconcile_channel', payload: { channel_id: '@chan' } })),
    ).toEqual({ kind: 'channel', id: '@chan' })
    expect(
      syncTarget(
        aTask({ task_type: 'reconcile_channel', status: 'running', payload: { channel_id: '@chan' } }),
      ),
    ).toBeNull()
    expect(syncTarget(aTask({ task_type: 'reconcile_playlist', payload: {} }))).toBeNull()
  })

  it('matches on the description, ignoring case', () => {
    const task = aTask({ task_type: 'reconcile_channel', payload: { channel_name: 'Rustaceans' } })

    expect(matchesTask(task, 'rustaceans')).toBe(true)
    expect(matchesTask(task, 'CHECK')).toBe(true)
    expect(matchesTask(task, 'nope')).toBe(false)
  })
})

describe('sectionOf', () => {
  const now = new Date('2026-11-14T20:03:00Z')

  it('puts a running task under running, even with a last error', () => {
    expect(sectionOf(aTask({ status: 'running', last_error: 'boom' }), now)).toBe('running')
  })

  it('puts a pending task with a last error under attention', () => {
    expect(
      sectionOf(aTask({ run_at: '2026-11-14T23:00:00Z', last_error: 'boom' }), now),
    ).toBe('attention')
  })

  it('puts a pending task due within the hour, or already due, under next hour', () => {
    const runAts = ['2026-11-14T21:03:00Z', '2026-11-14T20:20:00Z', '2026-11-14T19:00:00Z']

    expect(runAts.map((run_at) => sectionOf(aTask({ run_at }), now))).toEqual([
      'nextHour',
      'nextHour',
      'nextHour',
    ])
  })

  it('puts a pending task due in over an hour under later', () => {
    expect(sectionOf(aTask({ run_at: '2026-11-14T21:04:00Z' }), now)).toBe('later')
  })
})

describe('batchReconciles', () => {
  const channelCheck = (id: number, name: string, run_at: string) =>
    aTask({ id, task_type: 'reconcile_channel', run_at, payload: { channel_id: `@${name}`, channel_name: name } })
  const playlistCheck = (id: number, name: string, run_at: string) =>
    aTask({ id, task_type: 'reconcile_playlist', run_at, payload: { playlist_id: `PL${id}`, playlist_name: name } })

  it('groups reconciles each due within 10 minutes of the previous one into one batch', () => {
    const tasks = [
      playlistCheck(3, 'chess', '2026-11-14T20:22:00Z'),
      channelCheck(1, '3Blue1Brown', '2026-11-14T20:20:00Z'),
      channelCheck(2, 'Eric Rosen', '2026-11-14T20:21:00Z'),
    ]

    expect(batchReconciles(tasks)).toEqual([
      {
        kind: 'batch',
        tasks: [tasks[1], tasks[2], tasks[0]],
        runAt: '2026-11-14T20:20:00Z',
        sources: [
          { taskId: 1, name: '3Blue1Brown', kind: 'channel', sourceId: '@3Blue1Brown' },
          { taskId: 2, name: 'Eric Rosen', kind: 'channel', sourceId: '@Eric Rosen' },
          { taskId: 3, name: 'chess', kind: 'playlist', sourceId: 'PL3' },
        ],
      },
    ])
  })

  it('splits a chain where the gap is over 10 minutes', () => {
    const tasks = [
      channelCheck(1, 'A', '2026-11-14T20:00:00Z'),
      channelCheck(2, 'B', '2026-11-14T20:10:00Z'),
      channelCheck(3, 'C', '2026-11-14T20:20:01Z'),
      channelCheck(4, 'D', '2026-11-14T20:25:00Z'),
    ]

    expect(
      batchReconciles(tasks).map((entry) =>
        entry.kind === 'batch' ? entry.tasks.map((task) => task.id) : entry.task.id,
      ),
    ).toEqual([
      [1, 2],
      [3, 4],
    ])
  })

  it('keeps a lone reconcile and every other task as their own entries, in run-time order', () => {
    const download = aTask({ id: 2, task_type: 'download_video', run_at: '2026-11-14T20:05:00Z' })
    const lone = channelCheck(1, 'A', '2026-11-14T20:00:00Z')

    expect(batchReconciles([download, lone])).toEqual([
      { kind: 'task', task: lone },
      { kind: 'task', task: download },
    ])
  })

  it('never batches an errored or running reconcile', () => {
    const errored = { ...channelCheck(1, 'A', '2026-11-14T20:00:00Z'), last_error: 'boom' }
    const running = { ...channelCheck(2, 'B', '2026-11-14T20:01:00Z'), status: 'running' }
    const pending = channelCheck(3, 'C', '2026-11-14T20:02:00Z')

    expect(batchReconciles([errored, running, pending])).toEqual([
      { kind: 'task', task: errored },
      { kind: 'task', task: running },
      { kind: 'task', task: pending },
    ])
  })
})

describe('buildTaskList', () => {
  const now = new Date('2026-11-14T20:00:00Z')
  const soon = '2026-11-14T20:10:00Z'
  const later = '2026-11-14T23:00:00Z'
  const tasks = [
    aTask({ id: 1, task_type: 'download_video', status: 'running', payload: { video_title: 'Intro' } }),
    aTask({ id: 2, task_type: 'fetch_thumbnail', run_at: soon, payload: { video_title: 'Intro' } }),
    aTask({ id: 3, task_type: 'reconcile_channel', run_at: later, payload: { channel_id: '@a', channel_name: 'Jobava fans' } }),
    aTask({ id: 4, task_type: 'reconcile_playlist', run_at: later, payload: { playlist_id: 'PL1', playlist_name: 'Openings' } }),
    aTask({ id: 5, task_type: 'summarize_video', run_at: soon, payload: { video_title: 'Intro' } }),
    aTask({ id: 6, task_type: 'update_ytdlp', run_at: later }),
  ]
  const options = { filter: 'all' as const, query: '', aiEnabled: true, now }

  it('counts every chip, the families adding up to All', () => {
    expect(buildTaskList(tasks, options).chips).toEqual([
      { filter: 'all', count: 6 },
      { filter: 'downloads', count: 2 },
      { filter: 'syncs', count: 2 },
      { filter: 'ai', count: 1 },
      { filter: 'upkeep', count: 1 },
    ])
  })

  it('hides empty chips, the AI chip included while AI is off', () => {
    const chips = buildTaskList(tasks.slice(0, 2).concat(tasks[4]), { ...options, aiEnabled: false }).chips

    expect(chips).toEqual([
      { filter: 'all', count: 3 },
      { filter: 'downloads', count: 2 },
      { filter: 'upkeep', count: 1 },
    ])
  })

  it('lists sections in order, batching reconciles and dropping empty sections', () => {
    expect(buildTaskList(tasks, options).sections).toEqual([
      { id: 'running', entries: [{ kind: 'task', task: tasks[0] }] },
      {
        id: 'nextHour',
        entries: [
          { kind: 'task', task: tasks[1] },
          { kind: 'task', task: tasks[4] },
        ],
      },
      {
        id: 'later',
        entries: [
          {
            kind: 'batch',
            tasks: [tasks[2], tasks[3]],
            runAt: later,
            sources: [
              { taskId: 3, name: 'Jobava fans', kind: 'channel', sourceId: '@a' },
              { taskId: 4, name: 'Openings', kind: 'playlist', sourceId: 'PL1' },
            ],
          },
          { kind: 'task', task: tasks[5] },
        ],
      },
    ])
  })

  it('lists only the selected chip, keeping every chip', () => {
    const list = buildTaskList(tasks, { ...options, filter: 'downloads' })

    expect([list.chips.length, list.sections.map((section) => section.id)]).toEqual([
      5,
      ['running', 'nextHour'],
    ])
  })

  it('filters by description and by a batch source name, ignoring case', () => {
    const byDescription = buildTaskList(tasks, { ...options, query: 'YT-DLP' }).sections
    const bySource = buildTaskList(tasks, { ...options, query: 'jobava' }).sections

    expect([byDescription, bySource]).toEqual([
      [{ id: 'later', entries: [{ kind: 'task', task: tasks[5] }] }],
      [{ id: 'later', entries: [{ kind: 'task', task: tasks[2] }] }],
    ])
  })
})

describe('statusLine', () => {
  const now = new Date('2026-11-14T20:03:00Z')
  const pending = (count: number) =>
    Array.from({ length: count }, () => aTask({ task_type: 'update_ytdlp', run_at: '2026-11-14T23:00:00Z' }))

  it('names the next task and when it is due while nothing runs', () => {
    const tasks = [
      aTask({ task_type: 'update_ytdlp', run_at: '2026-11-14T22:00:00Z' }),
      aTask({ task_type: 'sync_plex_watch_state', run_at: '2026-11-14T20:05:00Z' }),
    ]

    expect(statusLine(tasks, now)).toBe('Nothing running. Next up in 2 min: Sync watch state with Plex.')
  })

  it('says the next task is starting soon once it is due', () => {
    const tasks = [aTask({ task_type: 'update_ytdlp', run_at: '2026-11-14T20:00:00Z' })]

    expect(statusLine(tasks, now)).toBe('Nothing running. Starting soon: Update yt-dlp.')
  })

  it('counts running downloads as videos', () => {
    const tasks = [aTask({ task_type: 'download_video', status: 'running' }), ...pending(71)]

    expect(statusLine(tasks, now)).toBe('Downloading 1 video. 71 more tasks are scheduled.')
  })

  it('counts running tasks of several kinds as tasks', () => {
    const tasks = [
      aTask({ task_type: 'download_video', status: 'running' }),
      aTask({ task_type: 'reconcile_channel', status: 'running' }),
      ...pending(10),
    ]

    expect(statusLine(tasks, now)).toBe('Running 2 tasks. 10 more tasks are scheduled.')
  })

  it('agrees in number for one scheduled task, and says nothing of none', () => {
    const downloads = [
      aTask({ task_type: 'download_video', status: 'running' }),
      aTask({ task_type: 'download_video', status: 'running' }),
    ]

    expect([
      statusLine([aTask({ task_type: 'update_ytdlp', status: 'running' }), ...pending(1)], now),
      statusLine(downloads, now),
    ]).toEqual(['Running 1 task. 1 more task is scheduled.', 'Downloading 2 videos.'])
  })
})
