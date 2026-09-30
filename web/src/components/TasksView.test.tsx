import { describe, expect, it } from 'vitest'
import { screen } from '@testing-library/react'
import { TasksView } from './TasksView'
import { aTask, mockApi, pendingForever, renderWithProviders } from '@/test/helpers'

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

  it('lists tasks with running ones first and badges per category', async () => {
    mockApi({
      'GET /api/tasks': [
        aTask({
          id: 1,
          task_type: 'download_video',
          status: 'pending',
          // Far in the future, so it categorizes as pending whenever this runs.
          run_at: '2999-01-01T00:00:00Z',
          payload: { video_title: 'Intro', channel_name: 'Chan' },
        }),
        aTask({
          id: 2,
          task_type: 'reconcile_channel',
          status: 'running',
          retries: 2,
          payload: { channel_name: 'Chan' },
        }),
      ],
    })

    renderWithProviders(<TasksView />)

    const items = await screen.findAllByRole('listitem')
    expect(items).toHaveLength(2)
    expect(items[0]).toHaveTextContent('Syncing channel Chan')
    expect(items[0]).toHaveTextContent('running')
    expect(items[0]).toHaveTextContent('2 retries')
    expect(items[1]).toHaveTextContent('Downloading Intro in Chan')
    expect(items[1]).toHaveTextContent('pending')
  })
})
