import { describe, expect, it } from 'vitest'
import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { VideoDetail } from './VideoDetail'
import { aChannel, aVideo, renderWithProviders } from '@/test/helpers'

describe('VideoDetail', () => {
  it('shows the title and a collapsed detail below the desktop breakpoint', () => {
    // The matchMedia stub reports a non-desktop viewport, so details start
    // collapsed.
    const video = aVideo({ title: 'My Video', description: 'The description.' })

    renderWithProviders(<VideoDetail basePath="playlists/mix" video={video} />)

    expect(screen.getByText('My Video')).toBeInTheDocument()
    expect(screen.queryByText('The description.')).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: /More/ })).toHaveAttribute('aria-expanded', 'false')
  })

  it('expands to the description, file path and YouTube link', async () => {
    const video = aVideo({
      id: 'abc',
      title: 'My Video',
      filename: 'my-video.mp4',
      description: 'The description.',
    })
    renderWithProviders(<VideoDetail basePath="playlists/mix" video={video} />)

    await userEvent.setup().click(screen.getByRole('button', { name: /More/ }))

    expect(screen.getByText('The description.')).toBeInTheDocument()
    expect(screen.getByText('playlists/mix/my-video.mp4')).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'Open on YouTube' })).toHaveAttribute(
      'href',
      'https://www.youtube.com/watch?v=abc',
    )
  })

  it('renders URLs in the description as links', async () => {
    const video = aVideo({
      description: 'See https://example.com/page. More text.',
    })
    renderWithProviders(<VideoDetail basePath="playlists/mix" video={video} />)

    await userEvent.setup().click(screen.getByRole('button', { name: /More/ }))

    // The trailing full stop stays outside the link.
    expect(screen.getByRole('link', { name: 'https://example.com/page' })).toHaveAttribute(
      'href',
      'https://example.com/page',
    )
  })

  it('badges a video that is not downloaded yet', async () => {
    const video = aVideo({ status: 'ERRORED_RETRYING', filename: null })
    renderWithProviders(<VideoDetail basePath="playlists/mix" video={video} />)

    await userEvent.setup().click(screen.getByRole('button', { name: /More/ }))

    expect(screen.getByText('Retrying')).toBeInTheDocument()
  })

  it('composes the meta line from the available parts', () => {
    const video = aVideo({
      channel_name: 'The Channel',
      published_at: '2026-01-02T00:00:00Z',
      synced_at: null,
    })
    renderWithProviders(<VideoDetail basePath="playlists/mix" video={video} />)

    // In a playlist view (no channel prop) the meta line names the channel.
    expect(screen.getByText('The Channel')).toBeInTheDocument()
    expect(screen.getByText(/Published/)).toBeInTheDocument()
    expect(screen.queryByText(/Synced/)).not.toBeInTheDocument()
  })

  it('links the channel avatar instead of naming the channel in a channel view', () => {
    const channel = aChannel({ id: 'chan', name: 'The Channel' })
    const video = aVideo({ channel_name: 'The Channel' })
    renderWithProviders(
      <VideoDetail basePath="channels/chan" video={video} channel={channel} />,
    )

    expect(screen.getByRole('link', { name: 'The Channel' })).toHaveAttribute(
      'href',
      '/channels/chan',
    )
    expect(screen.queryByText('The Channel', { selector: 'span' })).not.toBeInTheDocument()
  })
})
