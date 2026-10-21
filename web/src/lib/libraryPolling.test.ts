import { describe, expect, it } from 'vitest'
import { anyWorkInProgress } from './libraryPolling'

describe('anyWorkInProgress', () => {
  it('is true while any listed source has work in progress', () => {
    expect(anyWorkInProgress([{ work_in_progress: false }, { work_in_progress: true }])).toBe(true)
  })

  it('is false once every listed source has settled', () => {
    expect(anyWorkInProgress([{ work_in_progress: false }, { work_in_progress: false }])).toBe(
      false,
    )
  })

  it('is false before the listing has loaded', () => {
    expect(anyWorkInProgress(undefined)).toBe(false)
  })
})
