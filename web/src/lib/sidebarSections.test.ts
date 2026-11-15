import { describe, expect, it } from 'vitest'
import {
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

describe('sectionView', () => {
  it('lists every row when unfolded', () => {
    const items = Array.from({ length: 12 }, () => aChannel())

    const view = sectionView(items, { folded: false, activeId: null })

    expect(view).toEqual({ folded: false, headingActive: false, rows: items, hiddenCount: 0 })
  })

  it('shows nothing but the heading when folded', () => {
    const items = [aChannel(), aChannel()]

    const view = sectionView(items, { folded: true, activeId: null })

    expect(view).toEqual({ folded: true, headingActive: false, rows: [], hiddenCount: 0 })
  })

  it('marks the heading active when folded and the active row belongs to the section', () => {
    const items = [aChannel({ id: 'a' }), aChannel({ id: 'b' })]

    const view = sectionView(items, { folded: true, activeId: 'b' })

    expect(view).toEqual({ folded: true, headingActive: true, rows: [], hiddenCount: 0 })
  })

  it('does not mark the heading active when unfolded', () => {
    const items = [aChannel({ id: 'a' }), aChannel({ id: 'b' })]

    const view = sectionView(items, { folded: false, activeId: 'b' })

    expect(view).toEqual({ folded: false, headingActive: false, rows: items, hiddenCount: 0 })
  })

  it('does not mark the heading active when the active id is not in the section', () => {
    const items = [aChannel({ id: 'a' }), aChannel({ id: 'b' })]

    const view = sectionView(items, { folded: true, activeId: 'elsewhere' })

    expect(view).toEqual({ folded: true, headingActive: false, rows: [], hiddenCount: 0 })
  })
})

describe('sectionView with a limit', () => {
  const named = (count: number) =>
    Array.from({ length: count }, (_, index) => aChannel({ id: `c${index}`, name: `Chan ${index}` }))

  it('shows the first rows up to the limit and counts the rest as hidden', () => {
    const items = named(10)

    const view = sectionView(items, { folded: false, activeId: null, limit: 8 })

    expect(view.rows?.map((item) => item.id)).toEqual(items.slice(0, 8).map((item) => item.id))
    expect(view.hiddenCount).toBe(2)
  })

  it('shows every row once expanded', () => {
    const items = named(10)

    const view = sectionView(items, { folded: false, activeId: null, limit: 8, expanded: true })

    expect(view).toEqual({ folded: false, headingActive: false, rows: items, hiddenCount: 0 })
  })

  it('hides nothing when the section fits within the limit', () => {
    const items = named(8)

    expect(sectionView(items, { folded: false, activeId: null, limit: 8 }).hiddenCount).toBe(0)
  })

  it('keeps the viewed row past the limit in place', () => {
    const items = named(10)

    const view = sectionView(items, { folded: false, activeId: 'c9', limit: 8 })

    expect(view.rows?.map((item) => item.id)).toEqual([
      ...items.slice(0, 8).map((item) => item.id),
      'c9',
    ])
    expect(view.hiddenCount).toBe(1)
  })
})

describe('sectionView with a filter', () => {
  const items = [
    aChannel({ id: 'a', name: 'Eric Rosen' }),
    aChannel({ id: 'b', name: 'Big Think' }),
    aChannel({ id: 'c', name: 'Daniel Rensch' }),
  ]

  it('shows every match, ignoring case, past the limit', () => {
    const view = sectionView(items, { folded: false, activeId: null, filter: 'ROS', limit: 0 })

    expect(view).toEqual({ folded: false, headingActive: false, rows: [items[0]], hiddenCount: 0 })
  })

  it('shows the matches of a folded section', () => {
    const view = sectionView(items, { folded: true, activeId: null, filter: 'think' })

    expect(view.rows).toEqual([items[1]])
    expect(view.folded).toBe(false)
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
