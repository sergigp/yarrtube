import { describe, expect, it } from 'vitest'
import { intervalForm, intervalMinutes } from './reconcileInterval'

describe('intervalForm', () => {
  it('shows a whole number of hours in hours', () => {
    expect(intervalForm(180)).toEqual({ amount: '3', unit: 'hours' })
  })

  it('shows any other interval in minutes', () => {
    expect(intervalForm(90)).toEqual({ amount: '90', unit: 'minutes' })
  })
})

describe('intervalMinutes', () => {
  it('converts hours to minutes', () => {
    expect(intervalMinutes({ amount: '6', unit: 'hours' })).toBe(360)
  })

  it('is null for an empty, zero or fractional amount', () => {
    expect(intervalMinutes({ amount: '', unit: 'minutes' })).toBeNull()
    expect(intervalMinutes({ amount: '0', unit: 'minutes' })).toBeNull()
    expect(intervalMinutes({ amount: '1.5', unit: 'hours' })).toBeNull()
  })

  it('accepts up to 24 hours', () => {
    expect(intervalMinutes({ amount: '24', unit: 'hours' })).toBe(1440)
    expect(intervalMinutes({ amount: '1440', unit: 'minutes' })).toBe(1440)
  })

  it('is null for more than 24 hours', () => {
    expect(intervalMinutes({ amount: '25', unit: 'hours' })).toBeNull()
    expect(intervalMinutes({ amount: '1441', unit: 'minutes' })).toBeNull()
  })
})
