import { afterEach, describe, expect, it, vi } from 'vitest'
import { cleanup, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { AddChannelDialog } from './AddChannelDialog'
import {
  aCategory,
  aCategoryList,
  aChannel,
  aPlaylist,
  mockApi,
  renderWithProviders,
  type Routes,
} from '@/test/helpers'

const baseRoutes: Routes = {
  'GET /api/categories': aCategoryList(),
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
  afterEach(() => {
    window.localStorage.clear()
  })

  it('keeps a separate remembered parent per dialog', async () => {
    window.localStorage.setItem('yarrtube.save-to.playlist', 'playlists/kids')
    window.localStorage.setItem('yarrtube.save-to.channel', 'channels/science')
    renderDialog({
      'GET /api/playlists': [aPlaylist({ path: 'playlists/kids/contes' })],
      'GET /api/channels': [aChannel({ path: 'channels/science/veritasium' })],
    })

    expect(
      await screen.findByRole('radio', { name: 'channels/science/', checked: true }),
    ).toBeInTheDocument()
    expect(screen.queryByRole('radio', { name: 'playlists/kids/' })).not.toBeInTheDocument()
  })

  it('suggests only parents of tracked channels in the channel dialog', async () => {
    renderDialog({
      'GET /api/playlists': [aPlaylist({ path: 'playlists/kids/contes' })],
      'GET /api/channels': [aChannel({ path: 'channels/science/veritasium' })],
    })

    expect(await screen.findByRole('radio', { name: 'channels/science/' })).toBeInTheDocument()
    expect(screen.getByRole('radio', { name: 'channels/', checked: true })).toBeInTheDocument()
    expect(screen.queryByRole('radio', { name: 'playlists/kids/' })).not.toBeInTheDocument()
  })

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
      reconcile_interval_minutes: 180,
    })
  })

  it('creates the channel with the interval set under advanced options', async () => {
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
    await user.click(screen.getByRole('button', { name: /Advanced options/ }))
    const amount = screen.getByLabelText('Check for new videos every')
    await user.clear(amount)
    await user.type(amount, '30')
    await user.click(screen.getByRole('combobox', { name: 'Interval unit' }))
    await user.click(await screen.findByRole('option', { name: 'minutes' }))
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
      reconcile_interval_minutes: 30,
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

  it('adds a channel into the chosen categories, and reports categories that failed to save', async () => {
    const routes = (streamsResponse: unknown): Routes => ({
      'GET /api/categories': aCategoryList([
        aCategory({ id: 'chess', name: 'Chess', channels: ['@other'] }),
        aCategory({ id: 'cooking', name: 'Cooking' }),
        aCategory({ id: 'streams', name: 'Streams' }),
      ]),
      'GET /api/channels/preview?channel=%40chan': {
        id: '@chan',
        title: 'The Channel',
        avatar_url: null,
      },
      'POST /api/channels': aChannel({ id: '@chan' }),
      'PUT /api/categories/chess/members': null,
      'PUT /api/categories/streams/members': streamsResponse,
    })
    const addWithCategories = async () => {
      const user = userEvent.setup()
      await user.type(screen.getByLabelText('Channel Handle or URL'), '@chan')
      const field = screen.getByRole('combobox', { name: 'Categories' })
      await user.type(field, 'che')
      await user.click(await screen.findByRole('option', { name: 'Chess' }))
      await user.type(field, 'str')
      await user.click(await screen.findByRole('option', { name: 'Streams' }))
      const submit = screen.getByRole('button', { name: 'Create Channel' })
      await waitFor(() => expect(submit).toBeEnabled())
      await user.click(submit)
    }
    const saved = renderDialog(routes(null))
    await addWithCategories()
    await waitFor(() => expect(saved.onOpenChange).toHaveBeenCalledWith(false))
    const membersSent = saved.fetchMock.mock.calls
      .filter(([, init]) => (init as RequestInit | undefined)?.method === 'PUT')
      .map(([url, init]) => [url, JSON.parse((init as RequestInit).body as string)])
    cleanup()

    const failed = renderDialog(routes({ status: 500, error: 'disk full' }))
    await addWithCategories()

    expect(membersSent).toEqual([
      ['/api/categories/chess/members', { channels: ['@other', '@chan'], playlists: [] }],
      ['/api/categories/streams/members', { channels: ['@chan'], playlists: [] }],
    ])
    expect(
      await screen.findByText(
        'The channel was added, but its categories were not saved: disk full',
      ),
    ).toBeInTheDocument()
    expect(failed.onOpenChange).not.toHaveBeenCalled()
  })
})
