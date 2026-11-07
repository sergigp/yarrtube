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
  categorization_context_length: string
  categorization_temperature: string
  categorization_thinking: boolean
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
    categorization_context_length: String(preferences.categorization_context_length),
    categorization_temperature: String(preferences.categorization_temperature),
    categorization_thinking: preferences.categorization_thinking,
    categorization_prompt: preferences.categorization_prompt,
  }
}

/**
 * The fields of `form` that differ from `saved`, as a patch; without a
 * section's thinking when its model can't think (`summariesThinking` or
 * `categorizationThinking` false). A number field that doesn't hold one (empty included) is `NaN`, for the
 * caller to refuse: JSON would send it as `null`, which the daemon reads as
 * unchanged.
 */
export function changedFields(
  saved: Preferences,
  form: AiSettingsForm,
  {
    summariesThinking,
    categorizationThinking,
  }: { summariesThinking: boolean; categorizationThinking: boolean } = {
    summariesThinking: true,
    categorizationThinking: true,
  },
): PreferencesPatch {
  const current: PreferencesPatch = {
    ai_enabled: form.ai_enabled,
    ollama_url: form.ollama_url.trim(),
    summaries_enabled: form.summaries_enabled,
    summaries_model: form.summaries_model,
    summaries_context_length: toNumber(form.summaries_context_length),
    summaries_temperature: toNumber(form.summaries_temperature),
    summaries_thinking: summariesThinking ? form.summaries_thinking : saved.summaries_thinking,
    summaries_prompt: form.summaries_prompt,
    categorization_enabled: form.categorization_enabled,
    categorization_model: form.categorization_model,
    categorization_context_length: toNumber(form.categorization_context_length),
    categorization_temperature: toNumber(form.categorization_temperature),
    categorization_thinking: categorizationThinking
      ? form.categorization_thinking
      : saved.categorization_thinking,
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
  return [
    patch.summaries_context_length,
    patch.summaries_temperature,
    patch.categorization_context_length,
    patch.categorization_temperature,
  ].some((value) => Number.isNaN(value))
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
