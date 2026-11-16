import { describe, expect, it } from 'vitest'
import { sourceCategorizes } from './sourceAi'

describe('sourceCategorizes', () => {
  it('categorizes only with summaries and categorization both on', () => {
    const cases = [
      [false, false],
      [false, true],
      [true, false],
      [true, true],
    ].map(([summaries_enabled, categorization_enabled]) =>
      sourceCategorizes({ summaries_enabled, categorization_enabled }),
    )

    expect(cases).toEqual([false, false, false, true])
  })
})
