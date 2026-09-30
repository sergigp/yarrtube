import { describe, expect, it, vi } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { AddChannelDialog } from './AddChannelDialog'
import { aChannel, mockApi, renderWithProviders, type Routes } from '@/test/helpers'

const baseRoutes: Routes = {
  'GET /api/channels': [],
  'GET /api/playlists': [],
  'GET /api/directories': { root: '/videos', path: '', entries: [] },
  'GET /api/directories?path=channels': { root: '/videos', path: 'channels', entries: [] },
}

function renderDialog(routes: Routes) {
  const fetchMock = mockApi({ ...baseRoutes, ...routes })
  const onOpenChange = vi.fn()
  renderWithProviders(<AddChannelDialog open onOpenChange={onOpenChange} />)
  return { fetchMock, onOpenChange }
}

describe('AddChannelDialog', () => {
  it('looks the entered handle up and states the limit and destination', async () => {
    renderDialog({
      'GET /api/channels/preview?channel=%40chan': {
        id: '@chan',
        title: 'The Channel',
        avatar_url: null,
      },
    })
    const user = userEvent.setup()

    await user.type(screen.getByLabelText('Channel Handle or URL'), '@chan')

    expect(
      await screen.findByText(/The latest 3 videos from “The Channel” will be downloaded to/),
    ).toBeInTheDocument()
    // The folder derives from the handle, not the display title.
    expect(screen.getByText('/videos/channels/chan')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Create Channel' })).toBeEnabled()
  })

  it('creates the channel with the numeric video limit and closes', async () => {
    const { fetchMock, onOpenChange } = renderDialog({
      'GET /api/channels/preview?channel=%40chan': {
        id: '@chan',
        title: 'The Channel',
        avatar_url: null,
      },
      'POST /api/channels': aChannel(),
    })
    const user = userEvent.setup()

    await user.type(screen.getByLabelText('Channel Handle or URL'), '@chan')
    const submit = screen.getByRole('button', { name: 'Create Channel' })
    await waitFor(() => expect(submit).toBeEnabled())
    await user.click(submit)

    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false))
    const createCall = fetchMock.mock.calls.find(
      ([, init]) => (init as RequestInit | undefined)?.method === 'POST',
    ) as [string, RequestInit]
    expect(JSON.parse(createCall[1].body as string)).toEqual({
      channel: '@chan',
      quality: 'high',
      video_limit: 3,
      path: 'channels/chan',
    })
  })

  it('treats differently-cased handles as already tracked', async () => {
    renderDialog({
      'GET /api/channels': [aChannel({ id: '@CHAN', name: 'Existing Channel' })],
      'GET /api/channels/preview?channel=%40chan': {
        id: '@chan',
        title: 'The Channel',
        avatar_url: null,
      },
    })
    const user = userEvent.setup()

    await user.type(screen.getByLabelText('Channel Handle or URL'), '@chan')

    expect(await screen.findByText('Already added as “Existing Channel”')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Create Channel' })).toBeDisabled()
  })

  it('updates the stated limit as the video limit field changes', async () => {
    renderDialog({
      'GET /api/channels/preview?channel=%40chan': {
        id: '@chan',
        title: 'The Channel',
        avatar_url: null,
      },
    })
    const user = userEvent.setup()

    await user.type(screen.getByLabelText('Channel Handle or URL'), '@chan')
    await screen.findByText(/The latest 3 videos/)

    await user.click(screen.getByRole('button', { name: /Advanced options/ }))
    const limit = screen.getByLabelText('Video Limit')
    await user.clear(limit)
    await user.type(limit, '1')

    expect(
      await screen.findByText(/The latest video from “The Channel” will be downloaded to/),
    ).toBeInTheDocument()
  })

  it('shows the lookup failure and keeps submission blocked', async () => {
    renderDialog({
      'GET /api/channels/preview?channel=nope': { status: 404, error: 'channel not found' },
    })
    const user = userEvent.setup()

    await user.type(screen.getByLabelText('Channel Handle or URL'), 'nope')

    expect(await screen.findByText('channel not found')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Create Channel' })).toBeDisabled()
  })
})
