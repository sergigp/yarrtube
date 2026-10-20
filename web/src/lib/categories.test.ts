import { describe, expect, it } from 'vitest'
import {
  categoryIdsOf,
  membersOf,
  membershipChanges,
  orderCategories,
  sourceChips,
  stagedSummary,
  toggled,
  visibleChips,
  withMember,
  withMembers,
  withoutMember,
  type SourceRef,
} from './categories'
import { aCategory, aChannel, aPlaylist } from '@/test/helpers'

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

  it('builds chips per kind by name ignoring case, channels with avatars', () => {
    const channels = [
      aChannel({ id: '@Hikaru', name: 'Hikaru', avatar_filename: 'hikaru.jpg' }),
      aChannel({ id: '@chessbrah', name: 'chessbrah', avatar_filename: null }),
    ]
    const playlists = [
      aPlaylist({ id: 'PLopenings', name: 'Openings' }),
      aPlaylist({ id: 'PLendgames', name: 'endgames' }),
    ]

    expect(sourceChips(channels, playlists)).toEqual({
      channels: [
        { source: { kind: 'channel', id: '@chessbrah' }, name: 'chessbrah', avatarFilename: null },
        { source: { kind: 'channel', id: '@Hikaru' }, name: 'Hikaru', avatarFilename: 'hikaru.jpg' },
      ],
      playlists: [
        { source: { kind: 'playlist', id: 'PLendgames' }, name: 'endgames', avatarFilename: null },
        { source: { kind: 'playlist', id: 'PLopenings' }, name: 'Openings', avatarFilename: null },
      ],
    })
  })

  it('filters chips by search and state keeping their order', () => {
    const chips = sourceChips(
      [
        aChannel({ id: '@veritasium', name: 'Veritasium' }),
        aChannel({ id: '@hikaru', name: 'Hikaru' }),
        aChannel({ id: '@vsauce', name: 'Vsauce' }),
        aChannel({ id: '@overlord', name: 'Overlord' }),
      ],
      [],
    ).channels
    const staged = { channels: ['@veritasium', '@hikaru'], playlists: [] }
    const names = (filter: 'all' | 'in' | 'out', search: string) =>
      visibleChips(chips, staged, search, filter).map((chip) => chip.name)

    expect({
      all: names('all', ''),
      in: names('in', ''),
      out: names('out', ''),
      allVer: names('all', ' VER '),
      inVer: names('in', 'ver'),
      outVer: names('out', 'ver'),
    }).toEqual({
      all: ['Hikaru', 'Overlord', 'Veritasium', 'Vsauce'],
      in: ['Hikaru', 'Veritasium'],
      out: ['Overlord', 'Vsauce'],
      allVer: ['Overlord', 'Veritasium'],
      inVer: ['Veritasium'],
      outVer: ['Overlord'],
    })
  })

  it('toggles a source and summarises staged additions and removals', () => {
    const saved = membersOf(
      aCategory({ channels: ['@hikaru'], playlists: ['PLendgames'], unwatched_count: 3 }),
    )
    const staged = [
      { kind: 'channel', id: '@gotham' },
      { kind: 'channel', id: '@hikaru' },
      { kind: 'playlist', id: 'PLopenings' },
      { kind: 'channel', id: '@gotham' },
      { kind: 'channel', id: '@ludwig' },
    ].reduce((members, source) => toggled(members, source as SourceRef), saved)

    expect({ saved, staged, summary: stagedSummary(saved, staged) }).toEqual({
      saved: { channels: ['@hikaru'], playlists: ['PLendgames'] },
      staged: { channels: ['@ludwig'], playlists: ['PLendgames', 'PLopenings'] },
      summary: { added: 2, removed: 1 },
    })
  })
})
