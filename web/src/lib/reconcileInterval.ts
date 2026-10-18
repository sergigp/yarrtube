export type IntervalUnit = 'minutes' | 'hours'

/** The "Check for new videos every" field, as entered. */
export interface IntervalForm {
  amount: string
  unit: IntervalUnit
}

export const DEFAULT_INTERVAL_FORM: IntervalForm = { amount: '3', unit: 'hours' }

/** The longest interval the server accepts: 24 hours. */
export const MAX_INTERVAL_MINUTES = 24 * 60

/** How a stored interval shows in the field: hours when whole, minutes otherwise. */
export function intervalForm(minutes: number): IntervalForm {
  return minutes % 60 === 0
    ? { amount: String(minutes / 60), unit: 'hours' }
    : { amount: String(minutes), unit: 'minutes' }
}

/** The entered interval in minutes; `null` unless it is a whole number from 1 minute to 24 hours. */
export function intervalMinutes(form: IntervalForm): number | null {
  const amount = Number(form.amount)
  if (form.amount.trim() === '' || !Number.isInteger(amount) || amount < 1) return null
  const minutes = form.unit === 'hours' ? amount * 60 : amount
  return minutes > MAX_INTERVAL_MINUTES ? null : minutes
}
