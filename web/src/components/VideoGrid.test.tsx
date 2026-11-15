import { afterEach, describe, expect, it, vi } from 'vitest'
import { screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { VideoGrid } from './VideoGrid'
import { aHomeVideo, mockApi, renderWithProviders } from '@/test/helpers'

/** The card (list item) holding the video titled `title`. */
function card(title: string): HTMLElement {
  return screen.getByRole('link', { name: title }).closest('li') as HTMLElement
}

/** The meta line under a card's title, matched by its whole text. */
function metaLine(text: string): HTMLElement {
  return screen.getByText(
    (_, element) =>
      element?.getAttribute('data-slot') === 'card-meta' && element.textContent === text,
  )
}

const BETTATECH = {
  kind: 'channel' as const,
  id: '@bettatech',
  name: 'BettaTech',
  path: 'channels/bettatech',
  avatar_filename: 'bettatech.jpg',
}

const CHIPS = new Map([
  ['ai', { name: 'AI & Code', color: '#8b5cf6' }],
  ['chess', { name: 'Chess', color: '#0ea5e9' }],
  ['science', { name: 'Science', color: '#14b8a6' }],
])

describe('VideoGrid', () => {
  afterEach(() => {
    vi.useRealTimers()
  })

  it('draws cards frameless: no border, no background of their own', () => {
    mockApi({})
    renderWithProviders(<VideoGrid videos={[aHomeVideo({ title: 'Fresh Video' })]} />)

    expect(card('Fresh Video').style.background).toBe('')
    expect(card('Fresh Video').className.split(' ')).not.toContain('border')
    expect(
      card('Fresh Video')
        .className.split(' ')
        .filter((name) => /^(bg-|border)/.test(name)),
    ).toEqual([])
  })

  it('zooms the thumbnail within its rounded bounds and shows a play hint on hover', () => {
    mockApi({})
    renderWithProviders(<VideoGrid videos={[aHomeVideo({ title: 'Fresh Video' })]} />)

    const thumbnail = card('Fresh Video').querySelector('[data-slot="card-thumbnail"]')
    expect(thumbnail).toHaveClass('overflow-hidden', 'rounded-xl')
    expect(thumbnail?.firstElementChild).toHaveClass('group-hover/card:scale-[1.03]')
    expect(thumbnail?.querySelector('[aria-hidden="true"].group-hover\\/card\\:opacity-100')).not.toBeNull()
  })

  it('dims a watched card: a faded thumbnail, a "Watched" badge and a muted title', () => {
    mockApi({})
    renderWithProviders(<VideoGrid videos={[aHomeVideo({ title: 'Seen It', watched: true })]} />)

    const thumbnail = card('Seen It').querySelector('[data-slot="card-thumbnail"]')
    expect(thumbnail?.firstElementChild).toHaveClass('opacity-45')
    expect(within(card('Seen It')).getByRole('img', { name: 'Watched' })).toHaveTextContent(
      'Watched',
    )
    expect(screen.getByRole('link', { name: 'Seen It' })).toHaveClass('text-muted-foreground')
  })

  it('marks a video watched from its quick action, shown only on hover where the device can hover', async () => {
    const fetchMock = mockApi({ 'POST /api/videos/vid1/watched': {} })
    renderWithProviders(<VideoGrid videos={[aHomeVideo({ id: 'vid1', title: 'Fresh Video' })]} />)

    const quickAction = screen.getByRole('button', { name: 'Mark watched: Fresh Video' })
    expect(quickAction).toHaveClass('hidden', '[@media(hover:hover)]:flex', 'opacity-0')
    await userEvent.setup().click(quickAction)

    expect(fetchMock).toHaveBeenCalledWith(
      '/api/videos/vid1/watched',
      expect.objectContaining({ method: 'POST' }),
    )
  })

  it('offers no quick action on a watched video', () => {
    mockApi({})
    renderWithProviders(<VideoGrid videos={[aHomeVideo({ title: 'Seen It', watched: true })]} />)

    expect(screen.queryByRole('button', { name: /^Mark watched/ })).not.toBeInTheDocument()
  })

  it('shows the menu only on hover or focus where the device can hover, always on touch', () => {
    mockApi({})
    renderWithProviders(<VideoGrid videos={[aHomeVideo({ title: 'Fresh Video' })]} />)

    expect(screen.getByRole('button', { name: 'Actions for Fresh Video' })).toHaveClass(
      '[@media(hover:hover)]:opacity-0',
      '[@media(hover:hover)]:group-hover/card:opacity-100',
      '[@media(hover:hover)]:group-has-[:focus-visible]/card:opacity-100',
      '[@media(hover:hover)]:data-[state=open]:opacity-100',
    )
  })

  it("shows a channel card's avatar and its meta line: the channel's name and the video's age", () => {
    vi.useFakeTimers({ toFake: ['Date'] })
    vi.setSystemTime(new Date('2026-10-30T12:00:00Z'))
    mockApi({})
    renderWithProviders(
      <VideoGrid
        videos={[
          aHomeVideo({
            title: 'Rust in 100 seconds',
            source: BETTATECH,
            published_at: '2026-10-28T10:00:00Z',
          }),
        ]}
      />,
    )

    expect(metaLine('BettaTech · 2d ago')).toBeInTheDocument()
    // The avatar sits in the meta line, inside the one link to the channel.
    const channelLink = screen.getByRole('link', { name: 'BettaTech' })
    expect(channelLink.closest('[data-slot="card-meta"]')).not.toBeNull()
    expect(channelLink.querySelector('img')).toBeTruthy()
  })

  it("shows a playlist card's icon and its meta line: the playlist's name and the video's age", () => {
    vi.useFakeTimers({ toFake: ['Date'] })
    vi.setSystemTime(new Date('2026-10-30T12:00:00Z'))
    mockApi({})
    renderWithProviders(
      <VideoGrid
        videos={[
          aHomeVideo({
            title: 'The Minority Attack',
            source: {
              kind: 'playlist',
              id: 'PLmiddle',
              name: 'Chess Middlegames',
              path: 'playlists/middlegames',
              avatar_filename: null,
            },
            published_at: '2026-09-24T12:00:00Z',
          }),
        ]}
      />,
    )

    expect(metaLine('Chess Middlegames · 5w ago')).toBeInTheDocument()
    // One link to the playlist, a list icon in place of an avatar.
    const playlistLink = screen.getByRole('link', { name: 'Chess Middlegames' })
    expect(playlistLink).toHaveAttribute('href', '/playlists/PLmiddle')
    expect(playlistLink.querySelector('img')).toBeNull()
    expect(playlistLink.querySelector('svg')).not.toBeNull()
  })

  it("shows only the source's name on the meta line of a video with no publish date", () => {
    mockApi({})
    renderWithProviders(
      <VideoGrid
        videos={[
          aHomeVideo({
            title: 'Undated',
            source: BETTATECH,
            published_at: null,
          }),
        ]}
      />,
    )

    expect(metaLine('BettaTech')).toBeInTheDocument()
  })

  it("shows a video's category as a dot in its colour and its name", () => {
    mockApi({})
    renderWithProviders(
      <VideoGrid
        videos={[aHomeVideo({ title: 'Opening Traps', category_ids: ['chess'] })]}
        chips={CHIPS}
      />,
    )

    const tags = within(card('Opening Traps')).getAllByRole('listitem')
    expect(tags.map((tag) => tag.textContent)).toEqual(['Chess'])
    expect(tags[0].querySelector('[aria-hidden="true"]')).toHaveStyle({ background: '#0ea5e9' })
  })

  it('shows only the first category, then "+N" for the rest, on one line', () => {
    mockApi({})
    renderWithProviders(
      <VideoGrid
        videos={[aHomeVideo({ title: 'Engines', category_ids: ['ai', 'chess', 'science'] })]}
        chips={CHIPS}
      />,
    )

    expect(
      within(card('Engines'))
        .getAllByRole('listitem')
        .map((tag) => tag.textContent),
    ).toEqual(['AI & Code', '+2'])
    expect(screen.getByText('+2')).toHaveAttribute('title', 'Chess, Science')
  })

  it('shows no chip row for a video in no category', () => {
    mockApi({})
    renderWithProviders(
      <VideoGrid videos={[aHomeVideo({ title: 'Loose', category_ids: [] })]} chips={CHIPS} />,
    )

    expect(within(card('Loose')).queryByRole('list', { name: 'Categories' })).toBeNull()
  })

  it('shows no chips unless the caller asks for them', () => {
    mockApi({})
    renderWithProviders(
      <VideoGrid videos={[aHomeVideo({ title: 'Opening Traps', category_ids: ['chess'] })]} />,
    )

    expect(within(card('Opening Traps')).queryByRole('list', { name: 'Categories' })).toBeNull()
  })

  it('lays a shelf out as one row sized by the content area, swiping on phones', () => {
    mockApi({})
    const videos = Array.from({ length: 6 }, (_, index) => aHomeVideo({ title: `Video ${index}` }))
    renderWithProviders(<VideoGrid videos={videos} layout="shelf" />)

    expect(screen.getByRole('list')).toHaveClass('@xl:grid-cols-3', 'max-sm:flex', 'max-sm:snap-x')
    expect(card('Video 1')).not.toHaveClass('hidden')
    expect(card('Video 2')).toHaveClass('hidden', 'max-sm:block', '@xl:block', 'max-sm:w-[92%]')
    expect(card('Video 5')).toHaveClass('hidden', '@7xl:block')
  })

  it('lays a feed out as one column on phones', () => {
    mockApi({})
    renderWithProviders(<VideoGrid videos={[aHomeVideo({ title: 'Fresh' })]} layout="feed" />)

    expect(screen.getByRole('list')).toHaveClass('max-sm:grid-cols-1!', '@xl:grid-cols-3')
  })

  it('opens the video from anywhere on the card, through its one video link stretched over it', () => {
    mockApi({})
    renderWithProviders(<VideoGrid videos={[aHomeVideo({ id: 'vid1', title: 'Fresh Video' })]} />)

    const videoLinks = within(card('Fresh Video'))
      .getAllByRole('link')
      .filter((link) => link.getAttribute('href')?.includes('?video='))
    expect(videoLinks.map((link) => link.textContent)).toEqual(['Fresh Video'])
    expect(card('Fresh Video')).toHaveClass('relative')
    expect(videoLinks[0]).toHaveClass('after:absolute', 'after:inset-0')
  })

  it('keeps the source links and the menu above the stretched video link', () => {
    mockApi({})
    renderWithProviders(
      <VideoGrid
        videos={[aHomeVideo({ title: 'Rust in 100 seconds', source: BETTATECH })]}
        chips={CHIPS}
      />,
    )

    for (const control of [
      ...screen.getAllByRole('link', { name: 'BettaTech' }),
      screen.getByRole('button', { name: 'Actions for Rust in 100 seconds' }),
    ]) {
      expect(control).toHaveClass('relative', 'z-10')
    }
  })

  it("truncates a long source name before the video's age", () => {
    vi.useFakeTimers({ toFake: ['Date'] })
    vi.setSystemTime(new Date('2026-10-30T12:00:00Z'))
    mockApi({})
    renderWithProviders(
      <VideoGrid
        videos={[
          aHomeVideo({
            title: 'Rust in 100 seconds',
            source: BETTATECH,
            published_at: '2026-10-28T10:00:00Z',
          }),
        ]}
      />,
    )

    expect(screen.getByText('BettaTech', { selector: 'p a span' })).toHaveClass('truncate')
    expect(screen.getByText('· 2d ago', { exact: false })).toHaveClass('shrink-0')
  })
})
