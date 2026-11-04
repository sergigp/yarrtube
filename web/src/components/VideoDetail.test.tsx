import { describe, expect, it, vi } from 'vitest'
import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { VideoDetail } from './VideoDetail'
import { aChannel, aVideo, mockApi, renderWithProviders } from '@/test/helpers'

describe('VideoDetail', () => {
  it('shows the storage location with no More control', () => {
    // The matchMedia stub reports a non-desktop viewport.
    const video = aVideo({ title: 'My Video', filename: 'my-video.mp4' })

    renderWithProviders(
      <VideoDetail
        basePath="playlists/mix"
        video={video}
        playbackRate={1}
        onPlaybackRateChange={() => {}}
      />,
    )

    expect(screen.getByText('My Video')).toBeInTheDocument()
    expect(screen.getByText('playlists/mix')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /More/ })).not.toBeInTheDocument()
  })

  it('shows the AI summary', () => {
    const video = aVideo({ summary: 'What the video says.' })

    renderWithProviders(
      <VideoDetail
        basePath="playlists/mix"
        video={video}
        playbackRate={1}
        onPlaybackRateChange={() => {}}
      />,
    )

    expect(screen.getByRole('region', { name: 'AI summary' })).toHaveTextContent(
      'AI summaryWhat the video says.',
    )
  })

  it('shows no AI summary label when the video has none', () => {
    const video = aVideo({ summary: null })

    renderWithProviders(
      <VideoDetail
        basePath="playlists/mix"
        video={video}
        playbackRate={1}
        onPlaybackRateChange={() => {}}
      />,
    )

    expect(screen.queryByRole('region', { name: 'AI summary' })).not.toBeInTheDocument()
    expect(screen.queryByText('AI summary')).not.toBeInTheDocument()
  })

  it('links the video on YouTube from the title row, in a new tab', () => {
    const video = aVideo({ id: 'abc' })
    renderWithProviders(
      <VideoDetail
        basePath="playlists/mix"
        video={video}
        playbackRate={1}
        onPlaybackRateChange={() => {}}
      />,
    )

    const link = screen.getByRole('link', { name: 'Open video on YouTube' })
    expect(link).toHaveAttribute('href', 'https://www.youtube.com/watch?v=abc')
    expect(link).toHaveAttribute('target', '_blank')
  })

  it('shows the season folder as the storage location', () => {
    const video = aVideo({ filename: 'Season 2026/S2026E031501 - Title.mp4' })
    renderWithProviders(
      <VideoDetail
        basePath="channels/some-channel"
        video={video}
        playbackRate={1}
        onPlaybackRateChange={() => {}}
      />,
    )

    expect(screen.getByText('channels/some-channel/Season 2026')).toBeInTheDocument()
    expect(screen.queryByText(/S2026E031501/)).not.toBeInTheDocument()
  })

  it('shows the status of a video that is not downloaded', () => {
    const video = aVideo({ status: 'ERRORED_RETRYING', filename: null })
    renderWithProviders(
      <VideoDetail
        basePath="playlists/mix"
        video={video}
        playbackRate={1}
        onPlaybackRateChange={() => {}}
      />,
    )

    expect(screen.getByText('Retrying')).toBeInTheDocument()
  })

  it('composes the meta line from the available parts', () => {
    const video = aVideo({
      channel_name: 'The Channel',
      published_at: '2026-01-02T00:00:00Z',
      synced_at: null,
    })
    renderWithProviders(
      <VideoDetail
        basePath="playlists/mix"
        video={video}
        playbackRate={1}
        onPlaybackRateChange={() => {}}
      />,
    )

    // In a playlist view (no channel prop) the meta line names the channel.
    expect(screen.getByText('The Channel')).toBeInTheDocument()
    expect(screen.getByText(/Published/)).toBeInTheDocument()
    expect(screen.queryByText(/Synced/)).not.toBeInTheDocument()
  })

  it('links the channel avatar instead of naming the channel in a channel view', () => {
    const channel = aChannel({ id: 'chan', name: 'The Channel' })
    const video = aVideo({ channel_name: 'The Channel' })
    renderWithProviders(
      <VideoDetail
        basePath="channels/chan"
        video={video}
        channel={channel}
        playbackRate={1}
        onPlaybackRateChange={() => {}}
      />,
    )

    expect(screen.getByRole('link', { name: 'The Channel' })).toHaveAttribute(
      'href',
      '/channels/chan',
    )
    expect(screen.queryByText('The Channel', { selector: 'span' })).not.toBeInTheDocument()
  })
  it('marks the selected video watched from the detail pane', async () => {
    const markWatched = vi.fn(() => null)
    mockApi({ 'POST /api/videos/abc/watched': markWatched })
    const video = aVideo({ id: 'abc', title: 'My Video' })
    renderWithProviders(
      <VideoDetail
        basePath="playlists/mix"
        video={video}
        playbackRate={1}
        onPlaybackRateChange={() => {}}
      />,
    )

    await userEvent.setup().click(screen.getByRole('button', { name: 'Mark watched' }))

    expect(markWatched).toHaveBeenCalledOnce()
  })

  it('says a watched video is watched first on the meta line, with no mark button', () => {
    const video = aVideo({
      watched: true,
      published_at: '2026-01-02T00:00:00Z',
      synced_at: null,
    })
    renderWithProviders(
      <VideoDetail
        basePath="playlists/mix"
        video={video}
        playbackRate={1}
        onPlaybackRateChange={() => {}}
      />,
    )

    expect(screen.getByText(/Published/).parentElement).toHaveTextContent(/^Watched · Published/)
    expect(screen.queryByRole('button', { name: 'Mark watched' })).not.toBeInTheDocument()
  })

  it('disables the speed control until the video is downloaded', () => {
    const video = aVideo({ status: 'IN_PROGRESS', filename: null })

    renderWithProviders(
      <VideoDetail
        basePath="playlists/mix"
        video={video}
        playbackRate={1}
        onPlaybackRateChange={() => {}}
      />,
    )

    expect(screen.getByRole('button', { name: 'Playback speed' })).toBeDisabled()
  })
})
