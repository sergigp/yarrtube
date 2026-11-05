import type { OllamaModel, Preferences, PreferencesPatch } from '@/api/types'

/** The AI page's editable fields, as typed (numbers as text). */
export interface AiSettingsForm {
  ai_enabled: boolean
  ollama_url: string
  summaries_enabled: boolean
  summaries_model: string
  summaries_context_length: string
  summaries_temperature: string
  summaries_thinking: boolean
  summaries_prompt: string
  categorization_enabled: boolean
  categorization_model: string
  categorization_prompt: string
}

/** The form showing `preferences`. */
export function formFrom(preferences: Preferences): AiSettingsForm {
  return {
    ai_enabled: preferences.ai_enabled,
    ollama_url: preferences.ollama_url ?? '',
    summaries_enabled: preferences.summaries_enabled,
    summaries_model: preferences.summaries_model,
    summaries_context_length: String(preferences.summaries_context_length),
    summaries_temperature: String(preferences.summaries_temperature),
    summaries_thinking: preferences.summaries_thinking,
    summaries_prompt: preferences.summaries_prompt,
    categorization_enabled: preferences.categorization_enabled,
    categorization_model: preferences.categorization_model,
    categorization_prompt: preferences.categorization_prompt,
  }
}

/**
 * The fields of `form` that differ from `saved`, as a patch; without
 * `summaries_thinking` when the model can't think (`thinking: false`). A
 * number field that doesn't hold one (empty included) is `NaN`, for the
 * caller to refuse: JSON would send it as `null`, which the daemon reads as
 * unchanged.
 */
export function changedFields(
  saved: Preferences,
  form: AiSettingsForm,
  { thinking }: { thinking: boolean } = { thinking: true },
): PreferencesPatch {
  const current: PreferencesPatch = {
    ai_enabled: form.ai_enabled,
    ollama_url: form.ollama_url.trim(),
    summaries_enabled: form.summaries_enabled,
    summaries_model: form.summaries_model,
    summaries_context_length: toNumber(form.summaries_context_length),
    summaries_temperature: toNumber(form.summaries_temperature),
    summaries_thinking: thinking ? form.summaries_thinking : saved.summaries_thinking,
    summaries_prompt: form.summaries_prompt,
    categorization_enabled: form.categorization_enabled,
    categorization_model: form.categorization_model,
    categorization_prompt: form.categorization_prompt,
  }
  const before: PreferencesPatch = { ...saved, ollama_url: saved.ollama_url ?? '' }
  return Object.fromEntries(
    Object.entries(current).filter(
      ([key, value]) => !Object.is(value, before[key as keyof PreferencesPatch]),
    ),
  )
}

/** Whether `patch` has a number field that isn't one. */
export function hasInvalidNumber(patch: PreferencesPatch): boolean {
  return [patch.summaries_context_length, patch.summaries_temperature].some((value) =>
    Number.isNaN(value),
  )
}

function toNumber(text: string): number {
  return text.trim() === '' ? Number.NaN : Number(text)
}

/**
 * The choices for "Model": every model on the server, and `selected` marked
 * as not installed when the server doesn't have it.
 */
export function modelOptions(
  models: OllamaModel[],
  selected: string,
): { value: string; label: string }[] {
  const installed = models.map((model) => ({ value: model.name, label: model.name }))
  return models.some((model) => model.name === selected)
    ? installed
    : [...installed, { value: selected, label: `${selected} (not installed)` }]
}
