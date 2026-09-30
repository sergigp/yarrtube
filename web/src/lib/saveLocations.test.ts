import { describe, expect, it } from 'vitest'
import { deriveSaveCandidates } from './saveLocations'

describe('deriveSaveCandidates', () => {
  it('derives distinct parents with counts from item paths', () => {
    const candidates = deriveSaveCandidates('playlists', [
      { path: 'playlists/kids/contes' },
      { path: 'playlists/kids/fa-la-la' },
      { path: 'playlists/music/lofi' },
      { path: 'root-level-item' },
    ])

    expect(candidates).toEqual([
      { path: 'playlists', count: 0 },
      { path: 'playlists/kids', count: 2 },
      { path: 'playlists/music', count: 1 },
    ])
  })

  it('pins the default parent first even with zero occupants', () => {
    const candidates = deriveSaveCandidates('channels', [
      { path: 'channels/science/veritasium', created_at: '2026-02-01T00:00:00Z' },
    ])

    expect(candidates).toEqual([
      { path: 'channels', count: 0 },
      { path: 'channels/science', count: 1 },
    ])
  })
})
