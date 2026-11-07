/** The category dialog's name, color and AI prompt, as typed. */
export interface CategoryFields {
  name: string
  color: string
  prompt: string
}

/** The fields of `fields` that differ from `base`, as an update; prompt only when `withPrompt`. */
export function changedDetails(
  base: CategoryFields,
  fields: CategoryFields,
  { withPrompt }: { withPrompt: boolean },
): { name?: string; color?: string; ai_prompt?: string } {
  const name = fields.name.trim()
  const prompt = fields.prompt.trim()
  return {
    ...(name !== base.name && { name }),
    ...(fields.color !== base.color && { color: fields.color }),
    ...(withPrompt && prompt !== base.prompt && { ai_prompt: prompt }),
  }
}
