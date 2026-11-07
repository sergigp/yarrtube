/** The category dialog's name, color and AI prompt, as typed. */
export interface CategoryFields {
  name: string
  color: string
  prompt: string
}

/** The fields of `fields` that differ from `base`, as an update; prompt only when `withPrompt`. */
export function changedDetails(
  _base: CategoryFields,
  _fields: CategoryFields,
  _options: { withPrompt: boolean },
): { name?: string; color?: string; ai_prompt?: string } {
  return {}
}
