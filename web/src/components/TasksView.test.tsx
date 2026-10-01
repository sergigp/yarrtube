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
})
