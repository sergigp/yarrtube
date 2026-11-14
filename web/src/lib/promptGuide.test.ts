import { describe, expect, it } from 'vitest'
import SHIPPED_PROMPT from '../../../prompts/summaries/v3.md?raw'
import { PROMPT_VARIABLES } from './promptGuide'

describe('PROMPT_VARIABLES', () => {
  it('lists every variable the shipped prompt uses, once each, with a description', () => {
    const used = new Set([...SHIPPED_PROMPT.matchAll(/\{([a-z_]+)\}/g)].map(([, name]) => name))
    const listed = PROMPT_VARIABLES.map((variable) => variable.name)

    expect(new Set(listed).size).toBe(listed.length)
    expect(listed).toEqual(expect.arrayContaining([...used]))
    for (const variable of PROMPT_VARIABLES) {
      expect(variable.description, variable.name).not.toBe('')
    }
  })
})
