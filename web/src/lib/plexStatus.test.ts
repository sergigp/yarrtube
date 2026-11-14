import { describe, expect, it } from 'vitest'
import { aPlexSync } from '@/test/helpers'
import { isSyncing, plexVersionLabel } from './plexStatus'

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
