import { describe, expect, it, vi } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { VideoActionsMenu } from './VideoActionsMenu'
import { useChannels, useRecentVideos } from '@/api/queries'
import { mockApi, renderWithProviders } from '@/test/helpers'

/** Keeps the channel list and home videos observed, so a refetch of them shows up as a request. */
function LibraryObserver() {
  useChannels()
  useRecentVideos()
  return null
}

const EMPTY_HOME = { continue_watching: [], quick_watches: [], latest: [] }

describe('VideoActionsMenu', () => {
  it('marks the video watched when "Mark as watched" is chosen', async () => {
    const markWatched = vi.fn(() => null)
    const channels = vi.fn(() => [])
    const home = vi.fn(() => EMPTY_HOME)
    mockApi({
      'POST /api/videos/abc/watched': markWatched,
      'GET /api/channels': channels,
      'GET /api/videos/home': home,
    })
    renderWithProviders(
      <>
        <LibraryObserver />
        <VideoActionsMenu videoId="abc" title="My Video" markable />
      </>,
    )
    await waitFor(() => expect(home).toHaveBeenCalledOnce())

    const user = userEvent.setup()
    await user.click(screen.getByRole('button', { name: 'Actions for My Video' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Mark as watched' }))

    expect(markWatched).toHaveBeenCalledOnce()
    await waitFor(() => expect(channels).toHaveBeenCalledTimes(2))
    await waitFor(() => expect(home).toHaveBeenCalledTimes(2))
  })

  it('disables "Mark as watched" when not markable', async () => {
    mockApi({})
    renderWithProviders(<VideoActionsMenu videoId="abc" title="My Video" markable={false} />)

    const user = userEvent.setup()
    await user.click(screen.getByRole('button', { name: 'Actions for My Video' }))

    expect(await screen.findByRole('menuitem', { name: 'Mark as watched' })).toHaveAttribute(
      'aria-disabled',
      'true',
    )
  })
  it('alerts and leaves the video as it was when marking fails', async () => {
    const alert = vi.spyOn(window, 'alert').mockImplementation(() => {})
    const channels = vi.fn(() => [])
    const home = vi.fn(() => EMPTY_HOME)
    mockApi({
      'POST /api/videos/abc/watched': { status: 400, error: 'video abc is not downloaded' },
      'GET /api/channels': channels,
      'GET /api/videos/home': home,
    })
    renderWithProviders(
      <>
        <LibraryObserver />
        <VideoActionsMenu videoId="abc" title="My Video" markable />
      </>,
    )
    await waitFor(() => expect(home).toHaveBeenCalledOnce())

    const user = userEvent.setup()
    await user.click(screen.getByRole('button', { name: 'Actions for My Video' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Mark as watched' }))

    await waitFor(() =>
      expect(alert).toHaveBeenCalledWith(
        'Failed to mark "My Video" watched: video abc is not downloaded',
      ),
    )
    expect(channels).toHaveBeenCalledOnce()
    expect(home).toHaveBeenCalledOnce()
  })
})
