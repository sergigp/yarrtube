import { describe, expect, it } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Home } from './Home'
import type { HomeVideoSource } from '@/api/types'
import {
  aCategory,
  aCategoryList,
  aHomeCategorySection,
  aHomeVideo,
  aPlaylist,
  mockApi,
  renderWithProviders,
  type Routes,
} from '@/test/helpers'

/** Mocks the API with no categories unless `routes` says otherwise. */
function mockHomeApi(routes: Routes) {
  return mockApi({ 'GET /api/categories': aCategoryList(), ...routes })
}

describe('Home', () => {
  it('shows only the latest section while the others are empty', async () => {
    mockHomeApi({
      'GET /api/videos/home': {
        continue_watching: [],
        categories: [],
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
    mockHomeApi({
      'GET /api/videos/home': {
        continue_watching: [aHomeVideo({ title: 'Half Watched', position_seconds: 60 })],
        categories: [],
        quick_watches: [aHomeVideo({ title: 'Short One' })],
        latest: [aHomeVideo({ title: 'Fresh Video' })],
      },
    })

    renderWithProviders(<Home />)

    expect(await screen.findByText('Continue watching')).toBeInTheDocument()
    expect(screen.getByText('Quick watches')).toBeInTheDocument()
    expect(screen.getByText('Latest videos')).toBeInTheDocument()
  })

  it('shows a "More about" section per category between continue watching and quick watches', async () => {
    mockHomeApi({
      'GET /api/videos/home': {
        continue_watching: [aHomeVideo({ title: 'Half Watched', position_seconds: 60 })],
        categories: [
          aHomeCategorySection({
            id: 'science',
            name: 'Science',
            videos: [aHomeVideo({ title: 'Black Holes' })],
          }),
          aHomeCategorySection({
            id: 'chess',
            name: 'Chess',
            videos: [aHomeVideo({ title: 'Rook Endgames' })],
          }),
        ],
        quick_watches: [aHomeVideo({ title: 'Short One' })],
        latest: [aHomeVideo({ title: 'Fresh Video' })],
      },
    })

    renderWithProviders(<Home />)

    await screen.findByRole('heading', { name: 'More about Science' })
    expect(screen.getAllByRole('heading', { level: 2 }).map((h) => h.textContent)).toEqual([
      'Continue watching',
      'More about Science',
      'More about Chess',
      'Quick watches',
      'Latest videos',
    ])
    const chess = screen.getByRole('heading', { name: 'More about Chess' }).closest('section')
    expect(within(chess as HTMLElement).getByText('Rook Endgames')).toBeInTheDocument()
  })

  it('shows progress on in-progress cards in every section', async () => {
    mockHomeApi({
      'GET /api/videos/home': {
        continue_watching: [aHomeVideo({ position_seconds: 60, duration_seconds: 600 })],
        categories: [],
        quick_watches: [],
        latest: [
          aHomeVideo({ title: 'Fresh Video' }),
          aHomeVideo({ title: 'Started Elsewhere', position_seconds: 120, duration_seconds: 600 }),
        ],
      },
    })

    renderWithProviders(<Home />)

    const heading = await screen.findByRole('heading', { name: 'Latest videos' })
    const latest = heading.closest('section') as HTMLElement
    const cards = await within(latest).findAllByRole('listitem')
    const [fresh, started] = cards as [HTMLElement, HTMLElement]
    expect(within(started).getByRole('progressbar', { name: 'Watch progress' })).toHaveAttribute(
      'aria-valuenow',
      '20',
    )
    expect(within(started).getByText('8m left')).toBeInTheDocument()
    expect(within(fresh).queryByRole('progressbar')).not.toBeInTheDocument()
    expect(screen.getAllByRole('progressbar')).toHaveLength(2)
  })

  it('shows no progress for watched or durationless cards', async () => {
    mockHomeApi({
      'GET /api/videos/home': {
        continue_watching: [],
        categories: [],
        quick_watches: [],
        latest: [
          aHomeVideo({ title: 'Seen', watched: true, duration_seconds: 600 }),
          aHomeVideo({ title: 'Unknown Length', position_seconds: 60, duration_seconds: null }),
        ],
      },
    })

    renderWithProviders(<Home />)

    const [seen, unknownLength] = (await screen.findAllByRole('listitem')) as [
      HTMLElement,
      HTMLElement,
    ]
    expect(screen.queryByRole('progressbar')).not.toBeInTheDocument()
    expect(within(seen).getByText('10:00')).toBeInTheDocument()
    expect(within(unknownLength).queryByText(/left$|^\d+:\d{2}$/)).not.toBeInTheDocument()
  })

  it('links a video card to its source detail view with a deep link', async () => {
    mockHomeApi({
      'GET /api/videos/home': {
        continue_watching: [],
        categories: [],
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
    mockHomeApi({
      'GET /api/videos/home': { continue_watching: [], categories: [], quick_watches: [], latest: [] },
    })

    renderWithProviders(<Home />)

    expect(await screen.findByText('No videos synced yet.')).toBeInTheDocument()
  })

  it('reports a load failure in the latest section', async () => {
    mockHomeApi({ 'GET /api/videos/home': { status: 500, error: 'boom' } })

    renderWithProviders(<Home />)

    expect(await screen.findByText('Failed to load recent videos: boom')).toBeInTheDocument()
  })

  it('marks watched videos with a tick', async () => {
    mockHomeApi({
      'GET /api/videos/home': {
        continue_watching: [],
        categories: [],
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
    mockHomeApi({
      'GET /api/videos/home': () => ({
        continue_watching: marked ? [] : [halfWatched],
        categories: [],
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
    mockHomeApi({
      'GET /api/videos/home': () => ({
        continue_watching: [],
        categories: [],
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
    mockHomeApi({
      'GET /api/videos/home': {
        continue_watching: [],
        categories: [],
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

    expect((await screen.findAllByRole('menuitem')).map((item) => item.textContent)).toEqual([
      'Mark as watched',
    ])
  })

  it('tints each card by its categories', async () => {
    mockHomeApi({
      'GET /api/videos/home': {
        continue_watching: [],
        categories: [],
        quick_watches: [],
        latest: [
          aHomeVideo({ title: 'One', category_ids: ['chess'] }),
          aHomeVideo({ title: 'Two', category_ids: ['chess', 'streams'] }),
          aHomeVideo({ title: 'Four', category_ids: ['chess', 'streams', 'news', 'music'] }),
          aHomeVideo({ title: 'None', category_ids: [] }),
        ],
      },
      'GET /api/categories': aCategoryList([
        aCategory({ id: 'chess', color: '#d3ecf8' }),
        aCategory({ id: 'streams', color: '#fff1bf' }),
        aCategory({ id: 'news', color: '#e9def8' }),
        aCategory({ id: 'music', color: '#ffd9d4' }),
      ]),
    })

    renderWithProviders(<Home />)

    const card = async (title: string) =>
      (await screen.findByText(title)).closest('li') as HTMLElement
    await waitFor(async () => expect(await card('One')).toHaveStyle({ background: '#d3ecf880' }))
    // jsdom drops the gradient's `to bottom` (cardBackground's own test pins the exact value).
    expect((await card('Two')).style.background).toMatch(
      /^linear-gradient\(.*rgba\(211, 236, 248, 0\.5\d*\) 0% 50%, rgba\(255, 241, 191, 0\.5\d*\) 50% 100%\)$/,
    )
    expect(await card('Four')).toHaveStyle({ background: '#f2f2f280' })
    expect((await card('None')).style.background).toBe('')
  })
})
