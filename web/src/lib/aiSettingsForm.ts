import type { Preferences, PreferencesPatch } from '@/api/types'

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
  }
}

/**
 * The fields of `form` that differ from `saved`, as a patch. A number field
 * that doesn't hold one (empty included) is `NaN`, for the caller to refuse:
 * JSON would send it as `null`, which the daemon reads as unchanged.
 */
export function changedFields(saved: Preferences, form: AiSettingsForm): PreferencesPatch {
  const current: PreferencesPatch = {
    ai_enabled: form.ai_enabled,
    ollama_url: form.ollama_url,
    summaries_enabled: form.summaries_enabled,
    summaries_model: form.summaries_model,
    summaries_context_length: toNumber(form.summaries_context_length),
    summaries_temperature: toNumber(form.summaries_temperature),
    summaries_thinking: form.summaries_thinking,
    summaries_prompt: form.summaries_prompt,
  }
  const before: PreferencesPatch = { ...saved, ollama_url: saved.ollama_url ?? '' }
  return Object.fromEntries(
    Object.entries(current).filter(
      ([key, value]) => !Object.is(value, before[key as keyof PreferencesPatch]),
    ),
  )
}

function toNumber(text: string): number {
  return text.trim() === '' ? Number.NaN : Number(text)
}
