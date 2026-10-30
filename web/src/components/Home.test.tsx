import { describe, expect, it } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Home } from './Home'
import { queryKeys } from '@/api/queries'
import type { HomeVideoSource } from '@/api/types'
import {
  aCategory,
  aCategoryList,
  aHomeCategorySection,
  aHomeVideo,
  aPlaylist,
  aPreferences,
  mockApi,
  pendingForever,
  renderWithProviders,
  type Routes,
} from '@/test/helpers'

/** Mocks the API with no categories and default preferences unless `routes` says otherwise. */
function mockHomeApi(routes: Routes) {
  return mockApi({
    'GET /api/categories': aCategoryList(),
    'GET /api/preferences': aPreferences(),
    ...routes,
  })
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

  it('links a "More about" heading to its category', async () => {
    mockHomeApi({
      'GET /api/videos/home': {
        continue_watching: [],
        categories: [aHomeCategorySection({ id: 'chess', name: 'Chess' })],
        quick_watches: [],
        latest: [],
      },
    })

    renderWithProviders(<Home />)

    const heading = await screen.findByRole('heading', { name: 'More about Chess' })
    expect(within(heading).getByRole('link', { name: 'More about Chess' })).toHaveAttribute(
      'href',
      '/categories/chess',
    )
  })

  it('offers "See all" in a "More about" section, linking to its category', async () => {
    mockHomeApi({
      'GET /api/videos/home': {
        continue_watching: [],
        categories: [aHomeCategorySection({ id: 'chess', name: 'Chess' })],
        quick_watches: [],
        latest: [],
      },
    })

    renderWithProviders(<Home />)

    const heading = await screen.findByRole('heading', { name: 'More about Chess' })
    const section = heading.closest('section') as HTMLElement
    expect(within(section).getByRole('link', { name: 'See all' })).toHaveAttribute(
      'href',
      '/categories/chess',
    )
  })

  it('shows no "More about" section when the response has none', async () => {
    mockHomeApi({
      'GET /api/videos/home': {
        continue_watching: [],
        categories: [],
        quick_watches: [],
        latest: [aHomeVideo({ title: 'Fresh Video' })],
      },
    })

    renderWithProviders(<Home />)

    expect(await screen.findByText('Fresh Video')).toBeInTheDocument()
    expect(screen.queryByText(/^More about/)).not.toBeInTheDocument()
    expect(screen.queryByRole('link', { name: 'See all' })).not.toBeInTheDocument()
  })

  it('hides an empty "Latest videos" when other sections have videos', async () => {
    mockHomeApi({
      'GET /api/videos/home': {
        continue_watching: [],
        categories: [
          aHomeCategorySection({ name: 'Chess', videos: [aHomeVideo({ title: 'Rook Endgames' })] }),
        ],
        quick_watches: [],
        latest: [],
      },
    })

    renderWithProviders(<Home />)

    expect(await screen.findByText('Rook Endgames')).toBeInTheDocument()
    expect(screen.queryByText('Latest videos')).not.toBeInTheDocument()
    expect(screen.queryByText('No videos synced yet.')).not.toBeInTheDocument()
  })

  it('shows "No videos synced yet." when every section is empty', async () => {
    mockHomeApi({
      'GET /api/videos/home': {
        continue_watching: [],
        categories: [],
        quick_watches: [],
        latest: [],
      },
    })

    renderWithProviders(<Home />)

    expect(await screen.findByText('No videos synced yet.')).toBeInTheDocument()
    expect(screen.getByRole('heading', { name: 'Latest videos' })).toBeInTheDocument()
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

    await screen.findByText('Seen')
    const [seen, unknownLength] = screen.getAllByRole('listitem') as [
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
      'GET /api/videos/home': {
        continue_watching: [],
        categories: [],
        quick_watches: [],
        latest: [],
      },
    })

    renderWithProviders(<Home />)

    expect(await screen.findByText('No videos synced yet.')).toBeInTheDocument()
  })

  it('reports a load failure in the latest section', async () => {
    mockHomeApi({ 'GET /api/videos/home': { status: 500, error: 'boom' } })

    renderWithProviders(<Home />)

    expect(await screen.findByText('Failed to load recent videos: boom')).toBeInTheDocument()
  })

  it('reports a failed refresh in the latest section, hiding the stale sections', async () => {
    let failing = false
    mockHomeApi({
      'GET /api/videos/home': () =>
        failing
          ? { status: 500, error: 'boom' }
          : {
              continue_watching: [aHomeVideo({ title: 'Half Watched', position_seconds: 60 })],
              categories: [aHomeCategorySection({ name: 'Chess' })],
              quick_watches: [],
              latest: [],
            },
    })
    const { queryClient } = renderWithProviders(<Home />)
    await screen.findByText('Half Watched')

    failing = true
    await queryClient.refetchQueries()

    expect(await screen.findByText('Failed to load recent videos: boom')).toBeInTheDocument()
    expect(screen.queryByText('Continue watching')).not.toBeInTheDocument()
    expect(screen.queryByText('More about Chess')).not.toBeInTheDocument()
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

  it('shows no category chips when categories are off', async () => {
    const preferences = aPreferences({ categories_enabled: false })
    mockHomeApi({
      'GET /api/videos/home': {
        continue_watching: [],
        categories: [],
        quick_watches: [],
        latest: [aHomeVideo({ title: 'One', category_ids: ['chess'] })],
      },
      'GET /api/categories': aCategoryList([aCategory({ id: 'chess', name: 'Chess' })]),
      'GET /api/preferences': preferences,
    })

    const { queryClient } = renderWithProviders(<Home />)

    const card = (await screen.findByText('One')).closest('li') as HTMLElement
    await waitFor(() => expect(queryClient.getQueryData(queryKeys.preferences)).toEqual(preferences))
    await waitFor(() => expect(queryClient.getQueryData(queryKeys.categories)).toBeDefined())
    expect(within(card).queryByRole('list', { name: 'Categories' })).not.toBeInTheDocument()
  })

  it('shows no "Latest videos" while it is turned off', async () => {
    mockHomeApi({
      'GET /api/videos/home': {
        continue_watching: [aHomeVideo({ title: 'Half Watched', position_seconds: 60 })],
        categories: [],
        quick_watches: [],
        latest: [],
      },
      'GET /api/preferences': aPreferences({ home_show_latest: false }),
    })

    renderWithProviders(<Home />)

    expect(await screen.findByText('Half Watched')).toBeInTheDocument()
    expect(screen.queryByRole('heading', { name: 'Latest videos' })).not.toBeInTheDocument()
  })

  it('says there is nothing to show when latest is off and nothing else shows', async () => {
    mockHomeApi({
      'GET /api/videos/home': {
        continue_watching: [],
        categories: [],
        quick_watches: [],
        latest: [],
      },
      'GET /api/preferences': aPreferences({ home_show_latest: false }),
    })

    renderWithProviders(<Home />)

    expect(await screen.findByText(/Nothing to show here\./)).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'Preferences' })).toHaveAttribute('href', '/preferences')
    expect(screen.queryByRole('heading', { name: 'Latest videos' })).not.toBeInTheDocument()
    expect(screen.queryByText('No videos synced yet.')).not.toBeInTheDocument()
  })

  it('shows no "More about" section while categories are off, even from an older response', async () => {
    mockHomeApi({
      'GET /api/videos/home': {
        continue_watching: [],
        categories: [
          aHomeCategorySection({ id: 'chess', name: 'Chess', videos: [aHomeVideo({ title: 'Gambit' })] }),
        ],
        quick_watches: [],
        latest: [aHomeVideo({ title: 'Fresh Video' })],
      },
      'GET /api/preferences': aPreferences({ categories_enabled: false }),
    })

    const { queryClient } = renderWithProviders(<Home />)

    await waitFor(() =>
      expect(queryClient.getQueryData(queryKeys.preferences)).toEqual(
        aPreferences({ categories_enabled: false }),
      ),
    )
    expect(await screen.findByText('Fresh Video')).toBeInTheDocument()
    expect(screen.queryByRole('heading', { name: 'More about Chess' })).not.toBeInTheDocument()
    expect(screen.queryByText('Gambit')).not.toBeInTheDocument()
  })

  it('shows placeholder sections, marked busy and without headings, while loading', async () => {
    mockHomeApi({ 'GET /api/videos/home': pendingForever() })

    renderWithProviders(<Home />)

    const status = await screen.findByRole('status')
    expect(status).toHaveTextContent('Loading…')
    expect(status.closest('[aria-busy="true"]')).not.toBeNull()
    expect(screen.queryByRole('heading')).toBeNull()
    expect(screen.queryByText('Latest videos')).toBeNull()
  })

  it('replaces the placeholders with the sections once loaded, nothing busy', async () => {
    mockHomeApi({
      'GET /api/videos/home': {
        continue_watching: [],
        categories: [],
        quick_watches: [],
        latest: [aHomeVideo({ title: 'Fresh Video' })],
      },
    })

    renderWithProviders(<Home />)

    expect(await screen.findByRole('heading', { name: 'Latest videos' })).toBeInTheDocument()
    expect(screen.queryByRole('status')).toBeNull()
    expect(document.querySelector('[aria-busy="true"]')).toBeNull()
  })

  it('shows category chips on continue watching, quick watches and latest, not in category sections', async () => {
    const chessVideo = (title: string) => aHomeVideo({ title, category_ids: ['chess'] })
    mockHomeApi({
      'GET /api/videos/home': {
        continue_watching: [chessVideo('Half Watched')],
        categories: [
          aHomeCategorySection({
            id: 'chess',
            name: 'Chess',
            videos: [chessVideo('Rook Endgames')],
          }),
        ],
        quick_watches: [chessVideo('Short One')],
        latest: [chessVideo('Fresh Video')],
      },
      'GET /api/categories': aCategoryList([
        aCategory({ id: 'chess', name: 'Chess', color: '#0193bf' }),
      ]),
    })

    renderWithProviders(<Home />)

    const chipsOf = (title: string) =>
      within(screen.getByRole('link', { name: title }).closest('li') as HTMLElement)
        .queryAllByRole('listitem')
        .map((chip) => chip.textContent)
    await waitFor(() => expect(chipsOf('Half Watched')).toEqual(['Chess']))
    expect(chipsOf('Short One')).toEqual(['Chess'])
    expect(chipsOf('Fresh Video')).toEqual(['Chess'])
    expect(chipsOf('Rook Endgames')).toEqual([])
  })

  it('lays continue watching, category sections and quick watches out as shelves, latest as a feed', async () => {
    mockHomeApi({
      'GET /api/videos/home': {
        continue_watching: [aHomeVideo({ title: 'Half Watched' })],
        categories: [aHomeCategorySection({ name: 'Chess', videos: [aHomeVideo()] })],
        quick_watches: [aHomeVideo({ title: 'Short One' })],
        latest: [aHomeVideo({ title: 'Fresh Video' })],
      },
    })

    renderWithProviders(<Home />)

    const listOf = async (heading: string) =>
      within(
        (await screen.findByRole('heading', { name: heading })).closest('section') as HTMLElement,
      ).getAllByRole('list')[0]
    for (const shelf of ['Continue watching', 'More about Chess', 'Quick watches']) {
      expect(await listOf(shelf)).toHaveClass('max-sm:flex')
    }
    expect(await listOf('Latest videos')).toHaveClass('max-sm:grid-cols-1!')
  })
})
