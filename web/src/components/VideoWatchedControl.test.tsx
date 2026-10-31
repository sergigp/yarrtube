import { describe, expect, it, vi } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { VideoWatchedControl } from './VideoWatchedControl'
import { useChannels } from '@/api/queries'
import { mockApi, pendingForever, renderWithProviders } from '@/test/helpers'

/** Keeps the channel list observed, so a refetch of it shows up as a request. */
function LibraryObserver() {
  useChannels()
  return null
}

describe('VideoWatchedControl', () => {
  it('marks an unwatched video watched and refreshes the library', async () => {
    const markWatched = vi.fn(() => null)
    const channels = vi.fn(() => [])
    mockApi({
      'POST /api/videos/abc/watched': markWatched,
      'GET /api/channels': channels,
      'GET /api/videos/home': {
        continue_watching: [],
        categories: [],
        quick_watches: [],
        latest: [],
      },
    })
    renderWithProviders(
      <>
        <LibraryObserver />
        <VideoWatchedControl videoId="abc" title="My Video" watched={false} downloaded />
      </>,
    )
    await waitFor(() => expect(channels).toHaveBeenCalledOnce())

    await userEvent.setup().click(screen.getByRole('button', { name: 'Mark watched' }))

    expect(markWatched).toHaveBeenCalledOnce()
    await waitFor(() => expect(channels).toHaveBeenCalledTimes(2))
  })
  it('shows a watched video as a status, hinting that rewatching unmarks it', () => {
    mockApi({})
    renderWithProviders(<VideoWatchedControl videoId="abc" title="My Video" watched downloaded />)

    expect(screen.queryByRole('button')).not.toBeInTheDocument()
    expect(screen.getByText('Watched')).toHaveAttribute(
      'title',
      'Rewatch past 10% to mark it unwatched',
    )
  })
  it('shows nothing while the video has not finished downloading', () => {
    mockApi({})
    const { container } = renderWithProviders(
      <VideoWatchedControl videoId="abc" title="My Video" watched={false} downloaded={false} />,
    )

    expect(container).toBeEmptyDOMElement()
  })
  it('alerts and stays unwatched when marking fails', async () => {
    const alert = vi.spyOn(window, 'alert').mockImplementation(() => {})
    mockApi({
      'POST /api/videos/abc/watched': {
        status: 400,
        error: 'video abc is not downloaded',
      },
    })
    renderWithProviders(
      <VideoWatchedControl videoId="abc" title="My Video" watched={false} downloaded />,
    )

    await userEvent.setup().click(screen.getByRole('button', { name: 'Mark watched' }))

    await waitFor(() =>
      expect(alert).toHaveBeenCalledWith(
        'Failed to mark "My Video" watched: video abc is not downloaded',
      ),
    )
    expect(screen.getByRole('button', { name: 'Mark watched' })).toBeEnabled()
    alert.mockRestore()
  })
  it('is disabled while the mark is in flight', async () => {
    mockApi({ 'POST /api/videos/abc/watched': pendingForever() })
    renderWithProviders(
      <VideoWatchedControl videoId="abc" title="My Video" watched={false} downloaded />,
    )

    await userEvent.setup().click(screen.getByRole('button', { name: 'Mark watched' }))

    expect(screen.getByRole('button', { name: 'Mark watched' })).toBeDisabled()
  })
})
