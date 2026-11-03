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

/** The fields of `form` that differ from `saved`, as a patch. */
export function changedFields(_saved: Preferences, _form: AiSettingsForm): PreferencesPatch {
  return {}
}
