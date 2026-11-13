import type { AiProvider, Effort, OllamaModel, Preferences, PreferencesPatch } from '@/api/types'

/** The AI page's editable fields, as typed (numbers as text). */
export interface AiSettingsForm {
  ai_enabled: boolean
  ai_provider: AiProvider
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
  /** Auto: the daemon sizes the context from the categories. */
  categorization_context_length_auto: boolean
  categorization_temperature: string
  categorization_thinking: boolean
  categorization_prompt: string
  anthropic_summaries_model: string
  anthropic_summaries_effort: Effort
  anthropic_categorization_model: string
  anthropic_categorization_effort: Effort
}

/** The form showing `preferences`. */
export function formFrom(preferences: Preferences): AiSettingsForm {
  return {
    ai_enabled: preferences.ai_enabled,
    ai_provider: preferences.ai_provider,
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
    categorization_context_length_auto: preferences.categorization_context_length_is_auto,
    categorization_temperature: String(preferences.categorization_temperature),
    categorization_thinking: preferences.categorization_thinking,
    categorization_prompt: preferences.categorization_prompt,
    anthropic_summaries_model: preferences.anthropic_summaries_model,
    anthropic_summaries_effort: preferences.anthropic_summaries_effort,
    anthropic_categorization_model: preferences.anthropic_categorization_model,
    anthropic_categorization_effort: preferences.anthropic_categorization_effort,
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
    ai_provider: form.ai_provider,
    ollama_url: form.ollama_url.trim(),
    summaries_enabled: form.summaries_enabled,
    summaries_model: form.summaries_model,
    summaries_context_length: toNumber(form.summaries_context_length),
    summaries_temperature: toNumber(form.summaries_temperature),
    summaries_thinking: summariesThinking ? form.summaries_thinking : saved.summaries_thinking,
    summaries_prompt: form.summaries_prompt,
    categorization_enabled: form.categorization_enabled,
    categorization_model: form.categorization_model,
    categorization_temperature: toNumber(form.categorization_temperature),
    categorization_thinking: categorizationThinking
      ? form.categorization_thinking
      : saved.categorization_thinking,
    categorization_prompt: form.categorization_prompt,
    anthropic_summaries_model: form.anthropic_summaries_model,
    anthropic_summaries_effort: form.anthropic_summaries_effort,
    anthropic_categorization_model: form.anthropic_categorization_model,
    anthropic_categorization_effort: form.anthropic_categorization_effort,
  }
  const before: PreferencesPatch = { ...saved, ollama_url: saved.ollama_url ?? '' }
  const changed: PreferencesPatch = Object.fromEntries(
    Object.entries(current).filter(
      ([key, value]) => !Object.is(value, before[key as keyof PreferencesPatch]),
    ),
  )
  const contextLength = changedContextLength(saved, form)
  return contextLength === undefined
    ? changed
    : { ...changed, categorization_context_length: contextLength }
}

/**
 * The categorization context length to send: `null` to go back to Auto, the
 * number shown once Auto is turned off (even unedited, to keep it), or
 * `undefined` when nothing changed.
 */
function changedContextLength(saved: Preferences, form: AiSettingsForm): number | null | undefined {
  if (form.categorization_context_length_auto) {
    return saved.categorization_context_length_is_auto ? undefined : null
  }
  const typed = toNumber(form.categorization_context_length)
  return !saved.categorization_context_length_is_auto &&
    Object.is(typed, saved.categorization_context_length)
    ? undefined
    : typed
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
