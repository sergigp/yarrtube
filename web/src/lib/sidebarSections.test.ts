import { describe, expect, it } from 'vitest'
import {
  matchesSearch,
  orderByUnwatched,
  sectionView,
  shownUnwatchedCount,
  unwatchedTotal,
} from './sidebarSections'
import { aChannel, aPlaylist } from '@/test/helpers'

describe('orderByUnwatched', () => {
  it('puts channels with the most unwatched videos first', () => {
    const channels = [
      aChannel({ id: 'a', unwatched_count: 0 }),
      aChannel({ id: 'b', unwatched_count: 5 }),
      aChannel({ id: 'c', unwatched_count: 2 }),
    ]

    expect(orderByUnwatched(channels).map((channel) => channel.id)).toEqual(['b', 'c', 'a'])
  })

  it('keeps the incoming order among ties and does not mutate the input', () => {
    const channels = [
      aChannel({ id: 'a', unwatched_count: 1 }),
      aChannel({ id: 'b', unwatched_count: 1 }),
      aChannel({ id: 'c', unwatched_count: 0 }),
      aChannel({ id: 'd', unwatched_count: 0 }),
    ]

    const ordered = orderByUnwatched(channels)

    expect(ordered.map((channel) => channel.id)).toEqual(['a', 'b', 'c', 'd'])
    expect(channels.map((channel) => channel.id)).toEqual(['a', 'b', 'c', 'd'])
    expect(ordered).not.toBe(channels)
  })

  it('orders entries by shown unwatched count', () => {
    const playlists = [
      aPlaylist({ id: 'a', unwatched_count: 0 }),
      aPlaylist({ id: 'music', unwatched_count: 40, hide_unwatched_count: true }),
      aPlaylist({ id: 'courses', unwatched_count: 2 }),
      aPlaylist({ id: 'z', unwatched_count: 0 }),
    ]

    expect(orderByUnwatched(playlists).map((playlist) => playlist.id)).toEqual([
      'courses',
      'a',
      'music',
      'z',
    ])
  })
})

describe('shownUnwatchedCount', () => {
  it('shows no unwatched count for an entry that hides it', () => {
    expect(shownUnwatchedCount(aPlaylist({ unwatched_count: 3, hide_unwatched_count: true }))).toBe(0)
  })
})

describe('matchesSearch', () => {
  it('matches names case-insensitively by substring', () => {
    const channel = aChannel({ name: 'Cooking With Sergi' })

    expect(matchesSearch(channel, 'cooking')).toBe(true)
    expect(matchesSearch(channel, 'WITH')).toBe(true)
    expect(matchesSearch(channel, 'baking')).toBe(false)
  })
})

describe('sectionView', () => {
  it('lists every row when unfolded', () => {
    const items = Array.from({ length: 12 }, () => aChannel())

    const view = sectionView(items, { searchText: '', folded: false, activeId: null })

    expect(view).toEqual({ folded: false, headingActive: false, rows: items })
  })

  it('shows nothing but the heading when folded', () => {
    const items = [aChannel(), aChannel()]

    const view = sectionView(items, { searchText: '', folded: true, activeId: null })

    expect(view).toEqual({ folded: true, headingActive: false, rows: [] })
  })

  it('shows every match when folded but searching', () => {
    const veritasium = aChannel({ name: 'Veritasium' })
    const items = [veritasium, aChannel({ name: 'Kurzgesagt' })]

    const view = sectionView(items, { searchText: 'ver', folded: true, activeId: null })

    expect(view).toEqual({ folded: false, headingActive: false, rows: [veritasium] })
  })

  it('marks the heading active when folded and the active row belongs to the section', () => {
    const items = [aChannel({ id: 'a' }), aChannel({ id: 'b' })]

    const view = sectionView(items, { searchText: '', folded: true, activeId: 'b' })

    expect(view).toEqual({ folded: true, headingActive: true, rows: [] })
  })

  it('does not mark the heading active when unfolded', () => {
    const items = [aChannel({ id: 'a' }), aChannel({ id: 'b' })]

    const view = sectionView(items, { searchText: '', folded: false, activeId: 'b' })

    expect(view).toEqual({ folded: false, headingActive: false, rows: items })
  })

  it('does not mark the heading active when the active id is not in the section', () => {
    const items = [aChannel({ id: 'a' }), aChannel({ id: 'b' })]

    const view = sectionView(items, { searchText: '', folded: true, activeId: 'elsewhere' })

    expect(view).toEqual({ folded: true, headingActive: false, rows: [] })
  })
})

describe('unwatchedTotal', () => {
  it("sums the rows' unwatched counts, treating missing counts as 0", () => {
    const items = [
      aChannel({ unwatched_count: 3 }),
      aChannel({ unwatched_count: 0 }),
      aChannel({ unwatched_count: 20 }),
      aPlaylist(),
    ]

    expect(unwatchedTotal(items)).toBe(23)
  })
})
