import { describe, expect, it } from 'vitest'
import { plexVersionLabel } from './plexStatus'

describe('plexVersionLabel', () => {
  it('drops the build suffix', () => {
    expect(plexVersionLabel('1.43.5.11079-b3b13e506')).toBe('1.43.5.11079')
  })

  it('keeps a version without a suffix', () => {
    expect(plexVersionLabel('1.43.5.11079')).toBe('1.43.5.11079')
  })
})
