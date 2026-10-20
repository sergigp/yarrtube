import { describe, expect, it } from 'vitest'
import {
  categoryIdsOf,
  membershipChanges,
  orderCategories,
  withMember,
  withMembers,
  withoutMember,
} from './categories'
import { aCategory } from '@/test/helpers'

describe('categories', () => {
  it('orders unread first, most first, ties and caught-up by name ignoring case', () => {
    const categories = [
      aCategory({ name: 'Alpha', unwatched_count: 0 }),
      aCategory({ name: 'sports', unwatched_count: 4 }),
      aCategory({ name: 'Charlie', unwatched_count: 12 }),
      aCategory({ name: 'delta', unwatched_count: 0 }),
      aCategory({ name: 'Cooking', unwatched_count: 4 }),
      aCategory({ name: 'Bravo', unwatched_count: 0 }),
    ]

    const ordered = orderCategories(categories)

    expect(ordered.map((category) => category.name)).toEqual([
      'Charlie',
      'Cooking',
      'sports',
      'Alpha',
      'Bravo',
      'delta',
    ])
    expect(categories.map((category) => category.name)[0]).toBe('Alpha')
  })

  it('finds the categories of a source', () => {
    const categories = [
      aCategory({ id: 'chess', channels: ['@Hikaru'], playlists: ['PLendgames'] }),
      aCategory({ id: 'streams', channels: ['@Hikaru'] }),
      aCategory({ id: 'cooking', playlists: ['@Hikaru'] }),
    ]

    expect([
      categoryIdsOf(categories, { kind: 'channel', id: '@Hikaru' }),
      categoryIdsOf(categories, { kind: 'playlist', id: 'PLendgames' }),
      categoryIdsOf(categories, { kind: 'channel', id: '@Nobody' }),
    ]).toEqual([['chess', 'streams'], ['chess'], []])
  })

  it('computes membership changes: adds, removes, leaves unchanged ones out', () => {
    const categories = [
      aCategory({ id: 'chess', channels: ['@Hikaru'], playlists: ['PLendgames'] }),
      aCategory({ id: 'streams', channels: ['@Ludwig', '@Hikaru'], playlists: ['PLclips'] }),
      aCategory({ id: 'cooking', channels: ['@Babish'] }),
      aCategory({ id: 'science' }),
    ]

    const changes = membershipChanges(categories, { kind: 'channel', id: '@Hikaru' }, [
      'chess',
      'cooking',
    ])

    expect(changes).toEqual([
      { id: 'streams', members: { channels: ['@Ludwig'], playlists: ['PLclips'] } },
      { id: 'cooking', members: { channels: ['@Babish', '@Hikaru'], playlists: [] } },
    ])
  })

  it('adds and removes members without duplicates', () => {
    const chess = aCategory({ channels: ['@Hikaru'], playlists: ['PLendgames'] })

    expect([
      withMember(chess, { kind: 'channel', id: '@Hikaru' }),
      withMember(chess, { kind: 'playlist', id: 'PLopenings' }),
      withoutMember(chess, { kind: 'channel', id: '@Hikaru' }),
      withMembers(chess, [
        { kind: 'channel', id: '@GothamChess' },
        { kind: 'channel', id: '@Hikaru' },
        { kind: 'playlist', id: 'PLopenings' },
        { kind: 'channel', id: '@GothamChess' },
      ]),
    ]).toEqual([
      { channels: ['@Hikaru'], playlists: ['PLendgames'] },
      { channels: ['@Hikaru'], playlists: ['PLendgames', 'PLopenings'] },
      { channels: [], playlists: ['PLendgames'] },
      { channels: ['@Hikaru', '@GothamChess'], playlists: ['PLendgames', 'PLopenings'] },
    ])
  })
})
