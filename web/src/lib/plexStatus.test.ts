import { describe, expect, it } from 'vitest'
import { aPlexSync } from '@/test/helpers'
import { isSyncing, plexVersionLabel, relativeTime, syncStatusLine } from './plexStatus'

describe('plexVersionLabel', () => {
  it('drops the build suffix', () => {
    expect(plexVersionLabel('1.43.5.11079-b3b13e506')).toBe('1.43.5.11079')
  })

  it('keeps a version without a suffix', () => {
    expect(plexVersionLabel('1.43.5.11079')).toBe('1.43.5.11079')
  })
})

describe('isSyncing', () => {
  const requestedAt = new Date('2026-11-14T10:00:00Z')

  it('is syncing while the sync runs', () => {
    expect(isSyncing(aPlexSync({ running: true }), null)).toBe(true)
  })

  it('is syncing after a request until a run finishes after it', () => {
    expect(isSyncing(aPlexSync({ last_run_at: '2026-11-14T09:58:00Z' }), requestedAt)).toBe(true)
    expect(isSyncing(aPlexSync({ last_run_at: null }), requestedAt)).toBe(true)
    expect(isSyncing(aPlexSync({ last_run_at: '2026-11-14T10:00:03Z' }), requestedAt)).toBe(false)
  })

  it('is idle otherwise', () => {
    expect(isSyncing(aPlexSync({ running: false }), null)).toBe(false)
  })
})

describe('relativeTime', () => {
  const now = new Date('2026-11-14T10:00:00Z')
  const from = (ms: number) => new Date(now.getTime() + ms).toISOString()

  it('names past minutes', () => {
    expect(relativeTime(from(-2 * 60_000), now)).toBe('2 minutes ago')
  })

  it('names future minutes', () => {
    expect(relativeTime(from(3 * 60_000), now)).toBe('in 3 minutes')
  })

  it('uses the singular for one minute', () => {
    expect(relativeTime(from(-60_000), now)).toBe('1 minute ago')
  })

  it('names future seconds', () => {
    expect(relativeTime(from(30_000), now)).toBe('in 30 seconds')
  })

  it('names hours', () => {
    expect(relativeTime(from(-3 * 3_600_000), now)).toBe('3 hours ago')
  })

  it('names days', () => {
    expect(relativeTime(from(2 * 86_400_000), now)).toBe('in 2 days')
  })
})

describe('syncStatusLine', () => {
  const at = new Date('2026-11-14T10:03:00Z')

  it('puts last and next run on one line', () => {
    expect(
      syncStatusLine(
        aPlexSync({ last_run_at: '2026-11-14T10:00:00Z', next_run_at: '2026-11-14T10:05:00Z' }),
        at,
      ),
    ).toBe('Last run 3 minutes ago · Next in 2 minutes')
    expect(syncStatusLine(aPlexSync({ last_run_at: null, next_run_at: null }), at)).toBe(
      'Not run yet',
    )
  })

  it('says when a failed sync retries', () => {
    expect(
      syncStatusLine(
        aPlexSync({
          last_run_at: '2026-11-14T09:59:00Z',
          last_error: 'timed out',
          next_run_at: '2026-11-14T10:14:00Z',
        }),
        at,
      ),
    ).toBe('Failed 4 minutes ago: timed out. Retries in 11 minutes.')
  })
})
