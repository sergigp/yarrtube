import { describe, expect, it } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Home } from './Home'
import type { HomeVideoSource } from '@/api/types'
import { aHomeVideo, aPlaylist, mockApi, renderWithProviders } from '@/test/helpers'

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
  it('marking a continue-watching video watched removes it from the section', async () => {
    const halfWatched = aHomeVideo({ id: 'half', title: 'Half Watched', position_seconds: 60 })
    let marked = false
    mockApi({
      'GET /api/videos/home': () => ({
        continue_watching: marked ? [] : [halfWatched],
        quick_watches: [],
        latest: [aHomeVideo({ title: 'Fresh Video' })],
      }),
      'POST /api/videos/half/watched': () => {
        marked = true
        return null
      },
    })
    renderWithProviders(<Home />)

    const user = userEvent.setup()
    await user.click(await screen.findByRole('button', { name: 'Actions for Half Watched' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Mark as watched' }))

    await waitFor(() => expect(screen.queryByText('Half Watched')).not.toBeInTheDocument())
    expect(screen.queryByText('Continue watching')).not.toBeInTheDocument()
  })

  it("excluding a card's playlist from home removes its cards", async () => {
    const bluey: HomeVideoSource = {
      kind: 'playlist',
      id: 'bluey',
      name: 'Bluey',
      path: 'playlists/bluey',
      avatar_filename: null,
    }
    let excluded = false
    mockApi({
      'GET /api/videos/home': () => ({
        continue_watching: [],
        quick_watches: excluded ? [] : [aHomeVideo({ title: 'Bluey Short', source: bluey })],
        latest: [
          ...(excluded ? [] : [aHomeVideo({ title: 'Bluey Episode', source: bluey })]),
          aHomeVideo({ title: 'Fresh Video' }),
        ],
      }),
      'PATCH /api/playlists/bluey': () => {
        excluded = true
        return aPlaylist({ id: 'bluey', name: 'Bluey', exclude_from_home: true })
      },
    })
    renderWithProviders(<Home />)

    const user = userEvent.setup()
    await user.click(await screen.findByRole('button', { name: 'Actions for Bluey Episode' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Exclude "Bluey" from home' }))

    await waitFor(() => expect(screen.queryByText('Bluey Episode')).not.toBeInTheDocument())
    expect(screen.queryByText('Bluey Short')).not.toBeInTheDocument()
    expect(screen.getByText('Fresh Video')).toBeInTheDocument()
  })

  it('a channel card offers no exclude item', async () => {
    mockApi({
      'GET /api/videos/home': {
        continue_watching: [],
        quick_watches: [],
        latest: [
          aHomeVideo({
            title: 'Channel Video',
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

    const user = userEvent.setup()
    await user.click(await screen.findByRole('button', { name: 'Actions for Channel Video' }))

    expect(
      (await screen.findAllByRole('menuitem')).map((item) => item.textContent),
    ).toEqual(['Mark as watched'])
  })
})
