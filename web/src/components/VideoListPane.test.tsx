import { afterEach, describe, expect, it, vi } from 'vitest'
import { screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { VideoListPane } from './VideoListPane'
import { useChannelVideos } from '@/api/queries'
import { aVideo, mockApi, renderWithProviders, type Routes } from '@/test/helpers'
import type { Video } from '@/api/types'

function renderPane(videos: Video[] | undefined) {
  mockApi({})
  renderWithProviders(
    <VideoListPane
      noun="channel"
      basePathOf={() => 'channels/chan'}
      sourceNameOf={() => 'My Channel'}
      videos={videos}
      error={null}
      selectedVideoId={null}
      onSelect={() => {}}
    />,
  )
}

/** The queue of channel "chan", fetched like a detail view does, with `selected` selected. */
function ChannelQueue({ selected }: { selected: string }) {
  const { data, error } = useChannelVideos('chan')
  return (
    <VideoListPane
      noun="channel"
      basePathOf={() => 'channels/chan'}
      sourceNameOf={() => 'My Channel'}
      videos={data}
      error={error}
      selectedVideoId={selected}
      onSelect={() => {}}
    />
  )
}

function renderQueue(routes: Routes, selected: string) {
  mockApi(routes)
  renderWithProviders(<ChannelQueue selected={selected} />)
}

describe('VideoListPane', () => {
  afterEach(() => {
    vi.useRealTimers()
  })

  it('shows how long ago each video was published below its title', () => {
    vi.useFakeTimers({ toFake: ['Date'] })
    vi.setSystemTime(new Date('2026-10-30T12:00:00Z'))

    renderPane([aVideo({ title: 'Fresh Video', published_at: '2026-10-27T09:00:00Z' })])

    const row = screen.getByRole('button', { name: /^(?!Actions).*Fresh Video/ })
    expect(row).toHaveTextContent(/Fresh Video3d ago/)
  })

  it('says why a video that is not downloaded has no file', () => {
    renderPane([
      aVideo({ title: 'Pending', status: 'PENDING', filename: null }),
      aVideo({ title: 'Retrying', status: 'RETRYING', filename: null }),
      aVideo({ title: 'Failed', status: 'FAILED', filename: null }),
      aVideo({ title: 'Unavailable', status: 'UNAVAILABLE', filename: null }),
    ])

    expect(screen.getAllByRole('img').map((icon) => icon.getAttribute('aria-label'))).toEqual([
      'This video is pending.',
      'This video failed to download and will be retried.',
      'This video failed to download.',
      'This video is unavailable on YouTube.',
    ])
  })

  it('shows no date for a video without a publish date', () => {
    renderPane([aVideo({ title: 'Undated', published_at: null })])

    expect(screen.getByRole('button', { name: /^(?!Actions).*Undated/ })).not.toHaveTextContent(
      /ago|just now/,
    )
  })

  it('shows placeholder rows, marked busy, while the videos load', () => {
    renderPane(undefined)

    const status = screen.getByRole('status')
    expect(status).toHaveTextContent('Loading…')
    expect(status.closest('[aria-busy="true"]')).not.toBeNull()
    expect(screen.queryByText('Loading videos…')).not.toBeInTheDocument()
  })

  it("discards a row's video from its menu after confirming", async () => {
    const retrying = aVideo({ id: 'v1', title: 'First Video', status: 'RETRYING', filename: null })
    const selected = aVideo({ id: 'v2', title: 'Second Video' })
    let stored: Video[] = [retrying, selected]
    const discard = vi.fn(() => {
      stored = [selected]
      return null
    })
    renderQueue(
      {
        'GET /api/channels/chan/videos': () => stored,
        'POST /api/videos/v1/discard': discard,
      },
      'v2',
    )
    const user = userEvent.setup()

    await user.click(await screen.findByRole('button', { name: 'Actions for First Video' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Discard…' }))
    const dialog = await screen.findByRole('dialog', { name: 'Discard "First Video"?' })
    await user.click(within(dialog).getByRole('button', { name: 'Discard' }))

    expect(discard).toHaveBeenCalledOnce()
    await vi.waitFor(() =>
      expect(screen.queryByRole('button', { name: /^(?!Actions).*First Video/ })).not.toBeInTheDocument(),
    )
    expect(screen.getByRole('button', { name: /^(?!Actions).*Second Video/ })).toHaveAttribute(
      'aria-current',
      'true',
    )
  })

  it('offers no Discard… for a downloading row', async () => {
    renderPane([aVideo({ title: 'Downloading Video', status: 'IN_PROGRESS', filename: null })])
    const user = userEvent.setup()

    await user.click(screen.getByRole('button', { name: 'Actions for Downloading Video' }))

    expect(await screen.findByRole('menuitem', { name: 'Mark as watched' })).toBeInTheDocument()
    expect(screen.queryByRole('menuitem', { name: 'Discard…' })).not.toBeInTheDocument()
  })
})
