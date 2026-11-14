import { describe, expect, it } from 'vitest'
import { changedSettingNames, unsavedChangesLabel } from './aiChanges'

describe('aiChanges', () => {
  it('names changed settings once each, in page order', () => {
    expect(
      changedSettingNames({
        categorization_prompt: 'p',
        summaries_temperature: 0.5,
        anthropic_summaries_model: 'claude-haiku-5-5',
        summaries_model: 'qwen3:4b',
        ai_enabled: true,
      }),
    ).toEqual(['AI features', 'Summaries model', 'Summaries temperature', 'Categorization prompt'])
  })

  it('counts unsaved changes in singular and plural', () => {
    expect(unsavedChangesLabel(1)).toBe('1 unsaved change')
    expect(unsavedChangesLabel(2)).toBe('2 unsaved changes')
  })
})
