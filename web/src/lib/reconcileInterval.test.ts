import { describe, expect, it } from 'vitest'
import { intervalForm } from './reconcileInterval'

describe('intervalForm', () => {
  it('shows a whole number of hours in hours', () => {
    expect(intervalForm(180)).toEqual({ amount: '3', unit: 'hours' })
  })

  it('shows any other interval in minutes', () => {
    expect(intervalForm(90)).toEqual({ amount: '90', unit: 'minutes' })
  })
})
