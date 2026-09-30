import { describe, expect, it } from 'vitest'
import {
  CAUGHT_UP_PREVIEW,
  UNREAD_CAP,
  channelLeadCount,
  collapse,
  matchesSearch,
  orderChannels,
} from './sidebarSections'
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

describe('collapse', () => {
  const rows = [{ id: 'a' }, { id: 'b' }, { id: 'c' }, { id: 'd' }]

  it('shows the leading rows and counts the hidden rest', () => {
    expect(collapse(rows, 2, null)).toEqual({
      shown: [{ id: 'a' }, { id: 'b' }],
      hiddenCount: 2,
    })
  })

  it('appends the active row when it falls outside the lead', () => {
    expect(collapse(rows, 2, 'd')).toEqual({
      shown: [{ id: 'a' }, { id: 'b' }, { id: 'd' }],
      hiddenCount: 1,
    })
  })

  it('does not duplicate an active row that is already in the lead', () => {
    expect(collapse(rows, 2, 'a')).toEqual({
      shown: [{ id: 'a' }, { id: 'b' }],
      hiddenCount: 2,
    })
  })

  it('hides nothing when the lead covers every row', () => {
    expect(collapse(rows, 10, null)).toEqual({ shown: rows, hiddenCount: 0 })
  })
})

describe('channelLeadCount', () => {
  it('leads with the unread channels', () => {
    const channels = [
      aChannel({ unwatched_count: 3 }),
      aChannel({ unwatched_count: 1 }),
      aChannel({ unwatched_count: 0 }),
    ]

    expect(channelLeadCount(channels)).toBe(2)
  })

  it('caps the lead at the unread cap', () => {
    const channels = Array.from({ length: UNREAD_CAP + 5 }, () =>
      aChannel({ unwatched_count: 1 }),
    )

    expect(channelLeadCount(channels)).toBe(UNREAD_CAP)
  })

  it('falls back to the caught-up preview when nothing is unread', () => {
    const channels = [aChannel({ unwatched_count: 0 }), aChannel({ unwatched_count: 0 })]

    expect(channelLeadCount(channels)).toBe(CAUGHT_UP_PREVIEW)
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
