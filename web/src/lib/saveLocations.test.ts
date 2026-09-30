import { describe, expect, it } from 'vitest'
import {
  deriveSaveCandidates,
  readRememberedParent,
  writeRememberedParent,
} from './saveLocations'

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

  it('orders by newest created_at, then count, then name, and applies the cap', () => {
    const candidates = deriveSaveCandidates(
      'playlists',
      [
        { path: 'playlists/older/a', created_at: '2026-01-01T00:00:00Z' },
        { path: 'playlists/newest/b', created_at: '2026-03-01T00:00:00Z' },
        // No created_at: falls back to count, then name, after dated parents.
        { path: 'playlists/undated-busy/c' },
        { path: 'playlists/undated-busy/d' },
        { path: 'playlists/undated-quiet/e' },
        { path: 'playlists/dropped/f', created_at: '2025-01-01T00:00:00Z' },
      ],
      5,
    )

    expect(candidates).toEqual([
      { path: 'playlists', count: 0 },
      { path: 'playlists/newest', count: 1 },
      { path: 'playlists/older', count: 1 },
      { path: 'playlists/dropped', count: 1 },
      { path: 'playlists/undated-busy', count: 2 },
    ])
  })
})

describe('remembered parent storage', () => {
  it('reads null and writes without throwing when storage is unavailable', () => {
    const original = Object.getOwnPropertyDescriptor(window, 'localStorage')
    const denied = () => {
      throw new Error('storage denied')
    }
    Object.defineProperty(window, 'localStorage', {
      configurable: true,
      get: denied,
    })

    try {
      expect(readRememberedParent('playlist')).toBeNull()
      expect(() => writeRememberedParent('playlist', 'playlists/kids')).not.toThrow()
    } finally {
      Object.defineProperty(window, 'localStorage', original!)
    }
  })
})
