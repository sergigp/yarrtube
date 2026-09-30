import { describe, expect, it } from 'vitest'
import { screen } from '@testing-library/react'
import { Home } from './Home'
import { aHomeVideo, mockApi, renderWithProviders } from '@/test/helpers'

describe('Home', () => {
  it('shows only the latest section while the others are empty', async () => {
    mockApi({
      'GET /api/videos/home': {
        continue_watching: [],
        quick_watches: [],
        latest: [aHomeVideo({ title: 'Fresh Video' })],
      },
    })

    renderWithProviders(<Home />)

    expect(await screen.findByText('Latest videos')).toBeInTheDocument()
    expect(await screen.findByText('Fresh Video')).toBeInTheDocument()
    expect(screen.queryByText('Continue watching')).not.toBeInTheDocument()
    expect(screen.queryByText('Quick watches')).not.toBeInTheDocument()
  })

  it('shows every section that has videos', async () => {
    mockApi({
      'GET /api/videos/home': {
        continue_watching: [aHomeVideo({ title: 'Half Watched', position_seconds: 60 })],
        quick_watches: [aHomeVideo({ title: 'Short One' })],
        latest: [aHomeVideo({ title: 'Fresh Video' })],
      },
    })

    renderWithProviders(<Home />)

    expect(await screen.findByText('Continue watching')).toBeInTheDocument()
    expect(screen.getByText('Quick watches')).toBeInTheDocument()
    expect(screen.getByText('Latest videos')).toBeInTheDocument()
    // Only the continue-watching section draws progress bars.
    expect(screen.getAllByRole('progressbar')).toHaveLength(1)
  })

  it('links a video card to its source detail view with a deep link', async () => {
    mockApi({
      'GET /api/videos/home': {
        continue_watching: [],
        quick_watches: [],
        latest: [
          aHomeVideo({
            id: 'vid 1',
            title: 'Fresh Video',
            source: {
              kind: 'channel',
              id: 'chan',
              name: 'The Channel',
              path: 'channels/chan',
              avatar_filename: null,
            },
          }),
        ],
      },
    })

    renderWithProviders(<Home />)

    const videoLink = await screen.findByRole('link', { name: 'Fresh Video' })
    expect(videoLink).toHaveAttribute('href', '/channels/chan?video=vid%201')
    // Two channel links per card: the avatar (aria-label) and the name.
    const channelLinks = screen.getAllByRole('link', { name: 'The Channel' })
    expect(channelLinks).toHaveLength(2)
    for (const link of channelLinks) {
      expect(link).toHaveAttribute('href', '/channels/chan')
    }
  })

  it('shows an empty message when nothing has synced yet', async () => {
    mockApi({
      'GET /api/videos/home': { continue_watching: [], quick_watches: [], latest: [] },
    })

    renderWithProviders(<Home />)

    expect(await screen.findByText('No videos synced yet.')).toBeInTheDocument()
  })

  it('reports a load failure in the latest section', async () => {
    mockApi({ 'GET /api/videos/home': { status: 500, error: 'boom' } })

    renderWithProviders(<Home />)

    expect(await screen.findByText('Failed to load recent videos: boom')).toBeInTheDocument()
  })

  it('marks watched videos with a tick', async () => {
    mockApi({
      'GET /api/videos/home': {
        continue_watching: [],
        quick_watches: [],
        latest: [aHomeVideo({ title: 'Seen It', watched: true })],
      },
    })

    renderWithProviders(<Home />)

    expect(await screen.findByRole('img', { name: 'Watched' })).toBeInTheDocument()
  })
})
