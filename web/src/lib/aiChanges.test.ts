import { describe, expect, it } from 'vitest'
import { changedSettingNames } from './aiChanges'

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
})
