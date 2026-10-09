import { describe, expect, it, vi } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Route, Routes } from 'react-router-dom'
import { ChannelDetail } from './ChannelDetail'
import type { VideoQuality } from '@/api/types'
import { aChannel, aVideo, mockApi, renderWithProviders, type Routes as ApiRoutes } from '@/test/helpers'

function renderChannel(routes: ApiRoutes, route: string) {
  const fetchMock = mockApi(routes)
  renderWithProviders(
    <Routes>
      <Route path="/channels/:id" element={<ChannelDetail />} />
      <Route path="/" element={<p>home page</p>} />
    </Routes>,
    { route },
  )
  return { fetchMock }
}

/** The JSON bodies the page sent with `method` to `path`, in order. */
function sentBodies(fetchMock: ReturnType<typeof mockApi>, method: string, path: string) {
  return fetchMock.mock.calls
    .filter(([url, init]) => url === path && (init as RequestInit | undefined)?.method === method)
    .map(([, init]) => JSON.parse(((init as RequestInit).body as string | undefined) ?? 'null'))
}

describe('ChannelDetail', () => {
  it('shows the channel header with a video summary', async () => {
    const channel = aChannel({ id: 'chan', name: 'The Channel', unwatched_count: 1 })
    renderChannel(
      {
        'GET /api/channels': [channel],
        'GET /api/channels/chan/videos': [aVideo(), aVideo()],
      },
      '/channels/chan',
    )

    expect(await screen.findByText('The Channel')).toBeInTheDocument()
    expect(await screen.findByText('2 videos · 1 unwatched')).toBeInTheDocument()
  })

  it('plays the first video by default without autoplay', async () => {
    const channel = aChannel({ id: 'chan' })
    const first = aVideo({ title: 'First Video', filename: 'first.mp4' })
    renderChannel(
      {
        'GET /api/channels': [channel],
        'GET /api/channels/chan/videos': [first, aVideo({ title: 'Second Video' })],
      },
      '/channels/chan',
    )

    await screen.findByRole('heading', { name: 'First Video' })
    const video = document.querySelector('video')
    expect(video).not.toBeNull()
    expect(video?.getAttribute('src')).toBe(`/media/${channel.path}/first.mp4`)
    expect(video?.hasAttribute('autoplay')).toBe(false)
  })

  it('autoplays a deep-linked video', async () => {
    const channel = aChannel({ id: 'chan' })
    const linked = aVideo({ id: 'linked', title: 'Linked Video', filename: 'linked.mp4' })
    renderChannel(
      {
        'GET /api/channels': [channel],
        'GET /api/channels/chan/videos': [aVideo({ title: 'First Video' }), linked],
      },
      '/channels/chan?video=linked',
    )

    await screen.findByRole('heading', { name: 'Linked Video' })
    const video = document.querySelector('video')
    expect(video?.getAttribute('src')).toBe(`/media/${channel.path}/linked.mp4`)
    expect(video?.hasAttribute('autoplay')).toBe(true)
  })

  it('switches the player when a video is picked from the list', async () => {
    const channel = aChannel({ id: 'chan' })
    const second = aVideo({ title: 'Second Video', filename: 'second.mp4' })
    renderChannel(
      {
        'GET /api/channels': [channel],
        'GET /api/channels/chan/videos': [aVideo({ title: 'First Video' }), second],
      },
      '/channels/chan',
    )

    await userEvent
      .setup()
      .click(await screen.findByRole('button', { name: '2:00 Second Video' }))

    await waitFor(() =>
      expect(document.querySelector('video')?.getAttribute('src')).toBe(
        `/media/${channel.path}/second.mp4`,
      ),
    )
  })

  it("opening a row's menu does not change the selected video", async () => {
    const channel = aChannel({ id: 'chan' })
    renderChannel(
      {
        'GET /api/channels': [channel],
        'GET /api/channels/chan/videos': [
          aVideo({ title: 'First Video' }),
          aVideo({ title: 'Second Video' }),
        ],
      },
      '/channels/chan',
    )
    const user = userEvent.setup()

    await user.click(await screen.findByRole('button', { name: 'Actions for Second Video' }))

    expect(await screen.findByRole('menuitem', { name: 'Mark as watched' })).toBeInTheDocument()
    expect(screen.getByRole('heading', { name: 'First Video' })).toBeInTheDocument()
  })

  it('marking a row watched shows its tick', async () => {
    const channel = aChannel({ id: 'chan' })
    const first = aVideo({ title: 'First Video' })
    const second = aVideo({ id: 'second', title: 'Second Video' })
    let marked = false
    renderChannel(
      {
        'GET /api/channels': [channel],
        'GET /api/channels/chan/videos': () => [first, { ...second, watched: marked }],
        'POST /api/videos/second/watched': () => {
          marked = true
          return null
        },
      },
      '/channels/chan',
    )
    const user = userEvent.setup()

    await user.click(await screen.findByRole('button', { name: 'Actions for Second Video' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Mark as watched' }))

    // The row's tick comes first in its select button's accessible name.
    expect(
      await screen.findByRole('button', { name: /^Watched.*Second Video$/ }),
    ).toBeInTheDocument()
    expect(screen.getByRole('heading', { name: 'First Video' })).toBeInTheDocument()
  })

  it('the selected row shows no menu', async () => {
    const channel = aChannel({ id: 'chan' })
    renderChannel(
      {
        'GET /api/channels': [channel],
        'GET /api/channels/chan/videos': [
          aVideo({ title: 'First Video' }),
          aVideo({ title: 'Second Video' }),
        ],
      },
      '/channels/chan',
    )

    const list = await screen.findByRole('list')
    expect(
      within(list).queryByRole('button', { name: 'Actions for First Video' }),
    ).not.toBeInTheDocument()
    expect(within(list).getByRole('button', { name: 'Actions for Second Video' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Actions for First Video' })).toBeInTheDocument()
  })

  it('explains when the selected video is not downloaded yet', async () => {
    const channel = aChannel({ id: 'chan' })
    renderChannel(
      {
        'GET /api/channels': [channel],
        'GET /api/channels/chan/videos': [
          aVideo({ title: 'Pending Video', status: 'PENDING', filename: null }),
        ],
      },
      '/channels/chan',
    )

    expect(await screen.findByText('This video has not been downloaded yet.')).toBeInTheDocument()
    expect(document.querySelector('video')).toBeNull()
  })

  it('reports an unknown channel', async () => {
    renderChannel(
      {
        'GET /api/channels': [],
        'GET /api/channels/missing/videos': [],
      },
      '/channels/missing',
    )

    expect(await screen.findByText('Channel not found.')).toBeInTheDocument()
  })

  it('deletes the channel after confirmation and navigates home', async () => {
    const channel = aChannel({ id: 'chan', name: 'The Channel' })
    renderChannel(
      {
        'GET /api/channels': [channel],
        'GET /api/channels/chan/videos': [],
        'DELETE /api/channels/chan': null,
      },
      '/channels/chan',
    )
    const user = userEvent.setup()

    await user.click(await screen.findByRole('button', { name: 'More actions for The Channel' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Delete' }))
    const dialog = await screen.findByRole('dialog')
    expect(within(dialog).getByText('Delete "The Channel"?')).toBeInTheDocument()
    await user.click(within(dialog).getByRole('button', { name: 'Delete' }))

    expect(await screen.findByText('home page')).toBeInTheDocument()
  })

  it('marks the channel watched from its page header menu', async () => {
    const first = aVideo({ title: 'First Video' })
    const second = aVideo({ title: 'Second Video' })
    let marked = false
    renderChannel(
      {
        'GET /api/channels': () => [
          aChannel({ id: 'chan', name: 'The Channel', unwatched_count: marked ? 0 : 2 }),
        ],
        'GET /api/channels/chan/videos': () => [
          { ...first, watched: marked },
          { ...second, watched: marked },
        ],
        'POST /api/channels/chan/watched': () => {
          marked = true
          return null
        },
      },
      '/channels/chan',
    )
    const user = userEvent.setup()

    expect(await screen.findByText('2 videos · 2 unwatched')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'More actions for The Channel' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Mark all watched' }))

    expect(await screen.findByText('2 videos')).toBeInTheDocument()
    expect(
      await screen.findByRole('button', { name: /^Watched.*Second Video$/ }),
    ).toBeInTheDocument()
  })

  it("edits the channel's settings from its page header menu", async () => {
    let quality: VideoQuality = 'high'
    const { fetchMock } = renderChannel(
      {
        'GET /api/channels': () => [
          aChannel({ id: 'chan', name: 'The Channel', quality, video_limit: 5 }),
        ],
        'GET /api/channels/chan/videos': [aVideo()],
        'PATCH /api/channels/chan': () => {
          quality = 'low'
          return aChannel({ id: 'chan', quality: 'low', video_limit: 5 })
        },
      },
      '/channels/chan',
    )
    const user = userEvent.setup()

    await user.click(await screen.findByRole('button', { name: 'More actions for The Channel' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Edit settings' }))
    const dialog = await screen.findByRole('dialog', { name: 'Edit The Channel settings' })
    await user.click(within(dialog).getByRole('combobox', { name: /Video quality/ }))
    await user.click(await screen.findByRole('option', { name: 'Low' }))
    await user.click(within(dialog).getByRole('button', { name: 'Save' }))

    await waitFor(() => expect(dialog).not.toBeInTheDocument())
    expect(sentBodies(fetchMock, 'PATCH', '/api/channels/chan')).toEqual([{ quality: 'low' }])
    expect(sentBodies(fetchMock, 'POST', '/api/channels/chan/reconcile')).toEqual([])

    await user.click(screen.getByRole('button', { name: 'More actions for The Channel' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Edit settings' }))
    expect(
      within(await screen.findByRole('dialog', { name: 'Edit The Channel settings' })).getByRole('combobox', {
        name: /Video quality/,
      }),
    ).toHaveTextContent('Low')
  })

  it('syncs the channel after its video limit changes', async () => {
    const { fetchMock } = renderChannel(
      {
        'GET /api/channels': [aChannel({ id: 'chan', name: 'The Channel', video_limit: 5 })],
        'GET /api/channels/chan/videos': [aVideo()],
        'PATCH /api/channels/chan': aChannel({ id: 'chan', video_limit: 20 }),
        'POST /api/channels/chan/reconcile': null,
      },
      '/channels/chan',
    )
    const user = userEvent.setup()

    await user.click(await screen.findByRole('button', { name: 'More actions for The Channel' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Edit settings' }))
    const dialog = await screen.findByRole('dialog', { name: 'Edit The Channel settings' })
    const limit = within(dialog).getByLabelText('Video limit')
    await user.clear(limit)
    await user.type(limit, '20')
    await user.click(within(dialog).getByRole('button', { name: 'Save' }))

    await waitFor(() =>
      expect(sentBodies(fetchMock, 'POST', '/api/channels/chan/reconcile')).toHaveLength(1),
    )
    expect(sentBodies(fetchMock, 'PATCH', '/api/channels/chan')).toEqual([{ video_limit: 20 }])
    expect(dialog).not.toBeInTheDocument()
  })

  it('alerts when the sync after saving fails', async () => {
    const alert = vi.spyOn(window, 'alert').mockImplementation(() => {})
    const { fetchMock } = renderChannel(
      {
        'GET /api/channels': [aChannel({ id: 'chan', name: 'The Channel', video_limit: 5 })],
        'GET /api/channels/chan/videos': [aVideo()],
        'PATCH /api/channels/chan': aChannel({ id: 'chan', video_limit: 20 }),
        'POST /api/channels/chan/reconcile': { status: 502, error: 'yt-dlp failed' },
      },
      '/channels/chan',
    )
    const user = userEvent.setup()

    await user.click(await screen.findByRole('button', { name: 'More actions for The Channel' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Edit settings' }))
    const dialog = await screen.findByRole('dialog', { name: 'Edit The Channel settings' })
    const limit = within(dialog).getByLabelText('Video limit')
    await user.clear(limit)
    await user.type(limit, '20')
    await user.click(within(dialog).getByRole('button', { name: 'Save' }))

    await waitFor(() =>
      expect(alert).toHaveBeenCalledWith(
        'Saved the settings of "The Channel", but failed to sync it: yt-dlp failed',
      ),
    )
    expect(sentBodies(fetchMock, 'PATCH', '/api/channels/chan')).toEqual([{ video_limit: 20 }])
    expect(dialog).not.toBeInTheDocument()
  })
})
