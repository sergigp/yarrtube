export type IntervalUnit = 'minutes' | 'hours'

/** The "Check for new videos every" field, as entered. */
export interface IntervalForm {
  amount: string
  unit: IntervalUnit
}

export const DEFAULT_INTERVAL_FORM: IntervalForm = { amount: '3', unit: 'hours' }

/** How a stored interval shows in the field: hours when whole, minutes otherwise. */
export function intervalForm(minutes: number): IntervalForm {
  return minutes % 60 === 0
    ? { amount: String(minutes / 60), unit: 'hours' }
    : { amount: String(minutes), unit: 'minutes' }
}

/** The entered interval in minutes; `null` when it is not a whole number of at least 1. */
export function intervalMinutes(form: IntervalForm): number | null {
  return Number(form.amount)
}
