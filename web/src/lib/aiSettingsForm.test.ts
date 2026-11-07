import { describe, expect, it } from 'vitest'
import { changedFields, formFrom, hasInvalidNumber } from './aiSettingsForm'
import { aPreferences } from '@/test/helpers'

describe('changedFields', () => {
  it('is empty when nothing changed', () => {
    const saved = aPreferences()

    expect(changedFields(saved, formFrom(saved))).toEqual({})
  })

  it('has only the changed fields, numbers as numbers', () => {
    const saved = aPreferences()

    expect(
      changedFields(saved, {
        ...formFrom(saved),
        ai_enabled: true,
        ollama_url: 'http://mac.local:11434',
        summaries_temperature: '0.5',
        summaries_context_length: '8192',
      }),
    ).toEqual({
      ai_enabled: true,
      ollama_url: 'http://mac.local:11434',
      summaries_temperature: 0.5,
      summaries_context_length: 8192,
    })
  })

  it('leaves out an untouched prompt', () => {
    const saved = aPreferences({ summaries_prompt: 'custom', summaries_prompt_is_default: false })

    expect(changedFields(saved, { ...formFrom(saved), summaries_model: 'gemma3:4b' })).toEqual({
      summaries_model: 'gemma3:4b',
    })
  })

  it('has an edited prompt', () => {
    const saved = aPreferences()

    expect(
      changedFields(saved, { ...formFrom(saved), summaries_prompt: 'edited' }),
    ).toEqual({ summaries_prompt: 'edited' })
  })

  it('has the categorization fields only when changed', () => {
    const saved = aPreferences()
    const form = {
      ...formFrom(saved),
      categorization_enabled: true,
      categorization_prompt: 'Answer yes or no.',
    }

    expect(changedFields(saved, form)).toEqual({
      categorization_enabled: true,
      categorization_prompt: 'Answer yes or no.',
    })
  })

  it('has the categorization generation settings only when changed', () => {
    const saved = aPreferences()

    expect(
      changedFields(saved, {
        ...formFrom(saved),
        categorization_context_length: '8192',
        categorization_temperature: '0.5',
        categorization_thinking: false,
      }),
    ).toEqual({
      categorization_context_length: 8192,
      categorization_temperature: 0.5,
      categorization_thinking: false,
    })
  })

  it("leaves out the thinking of a model that can't think, section by section", () => {
    const saved = aPreferences()
    const form = { ...formFrom(saved), summaries_thinking: false, categorization_thinking: false }

    expect(
      changedFields(saved, form, { summariesThinking: true, categorizationThinking: false }),
    ).toEqual({ summaries_thinking: false })
    expect(
      changedFields(saved, form, { summariesThinking: false, categorizationThinking: true }),
    ).toEqual({ categorization_thinking: false })
  })

  it('sends the URL without surrounding spaces', () => {
    const saved = aPreferences({ ollama_url: 'http://mac.local:11434' })

    expect(changedFields(saved, { ...formFrom(saved), ollama_url: '  http://nas:11434 ' })).toEqual({
      ollama_url: 'http://nas:11434',
    })
    expect(
      changedFields(saved, { ...formFrom(saved), ollama_url: ' http://mac.local:11434 ' }),
    ).toEqual({})
  })

  it('unsets a cleared URL', () => {
    const saved = aPreferences({ ollama_url: 'http://mac.local:11434' })

    expect(changedFields(saved, { ...formFrom(saved), ollama_url: '' })).toEqual({
      ollama_url: '',
    })
  })

  it('marks a number that is not one as NaN', () => {
    const saved = aPreferences()

    expect(
      changedFields(saved, {
        ...formFrom(saved),
        summaries_context_length: 'lots',
        summaries_temperature: '',
      }),
    ).toEqual({ summaries_context_length: Number.NaN, summaries_temperature: Number.NaN })
  })
})

describe('hasInvalidNumber', () => {
  it('is false for numbers', () => {
    expect(
      hasInvalidNumber({ categorization_context_length: 4096, categorization_temperature: 0 }),
    ).toBe(false)
  })

  it('catches each number field', () => {
    expect(hasInvalidNumber({ summaries_context_length: Number.NaN })).toBe(true)
    expect(hasInvalidNumber({ summaries_temperature: Number.NaN })).toBe(true)
    expect(hasInvalidNumber({ categorization_context_length: Number.NaN })).toBe(true)
    expect(hasInvalidNumber({ categorization_temperature: Number.NaN })).toBe(true)
  })
})
