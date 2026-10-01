import { describe, expect, it } from 'vitest'
import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { TasksView } from './TasksView'
import { aTask, mockApi, pendingForever, renderWithProviders } from '@/test/helpers'

// A spread of tasks across every family, with one running task per the Active tab.
function aMixOfTasks() {
  return [
    aTask({
      id: 1,
      task_type: 'download_video',
      status: 'running',
      payload: { video_title: 'Intro', channel_name: 'Chan' },
    }),
    aTask({
      id: 2,
      task_type: 'fetch_thumbnail',
      status: 'pending',
      run_at: '2999-01-01T00:00:00Z',
      payload: { video_title: 'Intro', channel_name: 'Chan' },
    }),
    aTask({
      id: 3,
      task_type: 'reconcile_channel',
      status: 'pending',
      run_at: '2999-01-01T00:00:00Z',
      payload: { channel_name: 'Chan' },
    }),
    aTask({
      id: 4,
      task_type: 'delete_video_file',
      status: 'pending',
      run_at: '2999-01-01T00:00:00Z',
      payload: { filename: 'a.mp4' },
    }),
    aTask({
      id: 5,
      task_type: 'update_ytdlp',
      status: 'pending',
      run_at: '2999-01-01T00:00:00Z',
      payload: {},
    }),
  ]
}

describe('TasksView', () => {
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

  it('says so when there is nothing queued', async () => {
    mockApi({ 'GET /api/tasks': [] })

    renderWithProviders(<TasksView />)

    expect(await screen.findByText('No pending or in-progress tasks.')).toBeInTheDocument()
  })

  it('opens on the Active tab, listing only running tasks', async () => {
    mockApi({
      'GET /api/tasks': [
        aTask({
          id: 1,
          task_type: 'reconcile_channel',
          status: 'running',
          retries: 2,
          payload: { channel_name: 'Chan' },
        }),
        aTask({
          id: 2,
          task_type: 'download_video',
          status: 'pending',
          // Far in the future, so it categorizes as pending whenever this runs.
          run_at: '2999-01-01T00:00:00Z',
          payload: { video_title: 'Intro', channel_name: 'Chan' },
        }),
      ],
    })

    renderWithProviders(<TasksView />)

    const items = await screen.findAllByRole('listitem')
    expect(items).toHaveLength(1)
    expect(items[0]).toHaveTextContent('Syncing channel Chan')
    expect(items[0]).toHaveTextContent('running')
    expect(items[0]).toHaveTextContent('2 retries')
    expect(screen.queryByText('Downloading Intro in Chan')).not.toBeInTheDocument()
  })

  it('shows each tab trigger with its task count', async () => {
    mockApi({ 'GET /api/tasks': aMixOfTasks() })

    renderWithProviders(<TasksView />)

    expect(await screen.findByRole('tab', { name: /Active/ })).toHaveTextContent('1')
    expect(screen.getByRole('tab', { name: /Downloads/ })).toHaveTextContent('2')
    expect(screen.getByRole('tab', { name: /Syncs/ })).toHaveTextContent('1')
    expect(screen.getByRole('tab', { name: /Cleanup/ })).toHaveTextContent('1')
    expect(screen.getByRole('tab', { name: /All/ })).toHaveTextContent('5')
  })

  it('lists download and thumbnail tasks under Downloads, and no sync or cleanup tasks', async () => {
    const user = userEvent.setup()
    mockApi({ 'GET /api/tasks': aMixOfTasks() })

    renderWithProviders(<TasksView />)

    await user.click(await screen.findByRole('tab', { name: /Downloads/ }))

    expect(screen.getByText('Downloading Intro in Chan')).toBeInTheDocument()
    expect(screen.getByText('Fetching thumbnail of Intro in Chan')).toBeInTheDocument()
    expect(screen.queryByText('Syncing channel Chan')).not.toBeInTheDocument()
    expect(screen.queryByText('Removing file a.mp4')).not.toBeInTheDocument()
  })

  it('tells the user nothing is running on an empty Active tab while other tabs still list tasks', async () => {
    const user = userEvent.setup()
    mockApi({
      'GET /api/tasks': [
        aTask({
          id: 1,
          task_type: 'reconcile_channel',
          status: 'pending',
          run_at: '2999-01-01T00:00:00Z',
          payload: { channel_name: 'Chan' },
        }),
      ],
    })

    renderWithProviders(<TasksView />)

    expect(await screen.findByText('Nothing running right now')).toBeInTheDocument()
    expect(screen.queryByRole('listitem')).not.toBeInTheDocument()

    await user.click(screen.getByRole('tab', { name: /Syncs/ }))

    expect(screen.getByText('Syncing channel Chan')).toBeInTheDocument()
    expect(screen.queryByText('Nothing running right now')).not.toBeInTheDocument()
  })

  it('hides the search field at 15 tasks and shows it above 15', async () => {
    const fifteen = Array.from({ length: 15 }, (_, i) =>
      aTask({ id: i + 1, task_type: 'download_video', status: 'running' }),
    )
    mockApi({ 'GET /api/tasks': fifteen })
    const { unmount } = renderWithProviders(<TasksView />)

    expect(await screen.findByRole('tab', { name: /Active/ })).toHaveTextContent('15')
    expect(screen.queryByRole('searchbox', { name: 'Search tasks' })).not.toBeInTheDocument()

    unmount()

    const sixteen = Array.from({ length: 16 }, (_, i) =>
      aTask({ id: i + 1, task_type: 'download_video', status: 'running' }),
    )
    mockApi({ 'GET /api/tasks': sixteen })
    renderWithProviders(<TasksView />)

    expect(await screen.findByRole('searchbox', { name: 'Search tasks' })).toBeInTheDocument()
  })

  it('filters the current tab by description and says when nothing matches', async () => {
    const user = userEvent.setup()
    const tasks = Array.from({ length: 16 }, (_, i) =>
      aTask({
        id: i + 1,
        task_type: 'download_video',
        status: 'running',
        payload: { video_title: i === 0 ? 'Zeta' : 'Alpha' },
      }),
    )
    mockApi({ 'GET /api/tasks': tasks })

    renderWithProviders(<TasksView />)

    const search = await screen.findByRole('searchbox', { name: 'Search tasks' })

    await user.type(search, 'zeta')
    expect(screen.getAllByRole('listitem')).toHaveLength(1)
    expect(screen.getByText('Downloading Zeta in an unknown playlist or channel')).toBeInTheDocument()

    await user.clear(search)
    await user.type(search, 'nomatch')
    expect(screen.queryByRole('listitem')).not.toBeInTheDocument()
    expect(screen.getByText('Nothing matches')).toBeInTheDocument()
  })

  it('clears the search text when switching tabs', async () => {
    const user = userEvent.setup()
    const tasks = Array.from({ length: 16 }, (_, i) =>
      aTask({ id: i + 1, task_type: 'download_video', status: 'running' }),
    )
    mockApi({ 'GET /api/tasks': tasks })

    renderWithProviders(<TasksView />)

    await user.type(await screen.findByRole('searchbox', { name: 'Search tasks' }), 'alpha')
    expect(screen.getByRole('searchbox', { name: 'Search tasks' })).toHaveValue('alpha')

    await user.click(screen.getByRole('tab', { name: /All/ }))

    expect(screen.getByRole('searchbox', { name: 'Search tasks' })).toHaveValue('')
  })
})
