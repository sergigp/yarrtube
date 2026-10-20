import { describe, expect, it } from 'vitest'
import { matchesSearch, orderChannels, sectionView } from './sidebarSections'
import { aChannel } from '@/test/helpers'

describe('orderChannels', () => {
  it('puts channels with the most unwatched videos first', () => {
    const channels = [
      aChannel({ id: 'a', unwatched_count: 0 }),
      aChannel({ id: 'b', unwatched_count: 5 }),
      aChannel({ id: 'c', unwatched_count: 2 }),
    ]

    expect(orderChannels(channels).map((channel) => channel.id)).toEqual(['b', 'c', 'a'])
  })

  it('keeps the incoming order among ties and does not mutate the input', () => {
    const channels = [
      aChannel({ id: 'a', unwatched_count: 1 }),
      aChannel({ id: 'b', unwatched_count: 1 }),
      aChannel({ id: 'c', unwatched_count: 0 }),
      aChannel({ id: 'd', unwatched_count: 0 }),
    ]

    const ordered = orderChannels(channels)

    expect(ordered.map((channel) => channel.id)).toEqual(['a', 'b', 'c', 'd'])
    expect(channels.map((channel) => channel.id)).toEqual(['a', 'b', 'c', 'd'])
    expect(ordered).not.toBe(channels)
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
})
