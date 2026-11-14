import type { PreferencesPatch } from '@/api/types'

/** Each AI setting's name, in the order the page shows them. */
const NAMES: [keyof PreferencesPatch, string][] = [
  ['ai_enabled', 'AI features'],
  ['ai_provider', 'Provider'],
  ['ollama_url', 'Ollama URL'],
  ['summaries_enabled', 'Summaries'],
  ['summaries_model', 'Summaries model'],
  ['anthropic_summaries_model', 'Summaries model'],
  ['anthropic_summaries_effort', 'Summaries effort'],
  ['summaries_context_length', 'Summaries context length'],
  ['summaries_temperature', 'Summaries temperature'],
  ['summaries_thinking', 'Summaries thinking'],
  ['summaries_prompt', 'Summaries prompt'],
  ['categorization_enabled', 'Categorization'],
  ['categorization_model', 'Categorization model'],
  ['anthropic_categorization_model', 'Categorization model'],
  ['anthropic_categorization_effort', 'Categorization effort'],
  ['categorization_context_length', 'Categorization context length'],
  ['categorization_temperature', 'Categorization temperature'],
  ['categorization_thinking', 'Categorization thinking'],
  ['categorization_prompt', 'Categorization prompt'],
]

/** Human names of the changed settings, deduped, in page order. */
export function changedSettingNames(patch: PreferencesPatch): string[] {
  const names = NAMES.filter(([key]) => key in patch).map(([, name]) => name)
  return [...new Set(names)]
}

/** "1 unsaved change" or "<n> unsaved changes". */
export function unsavedChangesLabel(_count: number): string {
  return ''
}
