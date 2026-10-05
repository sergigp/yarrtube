import { describe, expect, it } from 'vitest'
import { errorMessage } from './errorMessage'

describe('errorMessage', () => {
  it("uses an Error's message", () => {
    expect(errorMessage(new Error('database is locked'))).toBe('database is locked')
  })

  it('stringifies anything else', () => {
    expect(errorMessage('timeout')).toBe('timeout')
    expect(errorMessage(404)).toBe('404')
  })
})
