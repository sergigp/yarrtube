import { afterEach, describe, expect, it, vi } from 'vitest'
import { screen, within } from '@testing-library/react'
import { VideoGrid } from './VideoGrid'
import { aHomeVideo, mockApi, renderWithProviders } from '@/test/helpers'
import { inkShade, softTint } from '@/lib/categoryColors'

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
  ['ai', { name: 'AI & Code', color: '#9870c5' }],
  ['chess', { name: 'Chess', color: '#0193bf' }],
  ['science', { name: 'Science', color: '#019b8d' }],
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

  it('shows a hover surface behind the card and zooms the thumbnail within its rounded bounds', () => {
    mockApi({})
    renderWithProviders(<VideoGrid videos={[aHomeVideo({ title: 'Fresh Video' })]} />)

    expect(card('Fresh Video')).toHaveClass('hover:bg-muted/60')
    const thumbnail = card('Fresh Video').querySelector('[data-slot="card-thumbnail"]')
    expect(thumbnail).toHaveClass('overflow-hidden', 'rounded-lg')
    expect(thumbnail?.firstElementChild).toHaveClass('group-hover/card:scale-[1.03]')
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
    expect(screen.getAllByRole('link', { name: 'BettaTech' })[0].querySelector('img')).toBeTruthy()
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
    expect(
      within(card('The Minority Attack')).getByRole('img', {
        name: 'Playlist',
      }),
    ).toBeInTheDocument()
    // The icon and the name both lead to the playlist, as a channel's avatar and name do.
    const playlistLinks = screen.getAllByRole('link', { name: 'Chess Middlegames' })
    expect(playlistLinks.map((link) => link.getAttribute('href'))).toEqual([
      '/playlists/PLmiddle',
      '/playlists/PLmiddle',
    ])
    expect(within(playlistLinks[0]).getByRole('img', { name: 'Playlist' })).toBeInTheDocument()
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

  it("shows a video's category as a chip in its hue: ink text over a soft tint", () => {
    mockApi({})
    renderWithProviders(
      <VideoGrid
        videos={[aHomeVideo({ title: 'Opening Traps', category_ids: ['chess'] })]}
        chips={CHIPS}
      />,
    )

    const chips = within(card('Opening Traps')).getAllByRole('listitem')
    expect(chips.map((chip) => chip.textContent)).toEqual(['Chess'])
    expect(chips[0]).toHaveStyle({ background: softTint('#0193bf'), color: inkShade('#0193bf') })
  })

  it('shows at most 2 chips, then a "+N" chip for the rest', () => {
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
        .map((chip) => chip.textContent),
    ).toEqual(['AI & Code', 'Chess', '+1'])
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
    expect(card('Video 2')).toHaveClass('hidden', 'max-sm:block', '@xl:block', 'max-sm:w-[75%]')
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
})
