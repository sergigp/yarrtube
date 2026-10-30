import { describe, expect, it } from 'vitest'
import { matchesSearch, searchByName } from './search'

describe('matchesSearch', () => {
  it('matches names case-insensitively by substring', () => {
    const channel = { name: 'Cooking With Sergi' }

    expect([
      matchesSearch(channel, 'cooking'),
      matchesSearch(channel, 'WITH'),
      matchesSearch(channel, 'baking'),
    ]).toEqual([true, true, false])
  })
})

describe('searchByName', () => {
  it('keeps the entries whose name contains the trimmed text, in their order', () => {
    const entries = [{ name: 'Chess' }, { name: 'Science' }, { name: 'ChessCoach Andras' }]

    expect(searchByName(entries, '  chess ')).toEqual([
      { name: 'Chess' },
      { name: 'ChessCoach Andras' },
    ])
  })

  it('finds nothing for blank text', () => {
    expect(searchByName([{ name: 'Chess' }], '   ')).toEqual([])
  })
})
