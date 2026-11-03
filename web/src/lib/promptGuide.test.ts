import { describe, expect, it } from 'vitest'
import SHIPPED_PROMPT from '../../../prompts/summaries/v3.md?raw'
import { PROMPT_SECTIONS } from './promptGuide'

/** Each `## heading` of the prompt with the `{variables}` its text uses. */
function shippedSections(): Map<string, Set<string>> {
  const sections = new Map<string, Set<string>>()
  let current: Set<string> | null = null
  for (const line of SHIPPED_PROMPT.split('\n')) {
    const heading = line.startsWith('## ') ? line.slice(3).trim() : null
    if (heading !== null) {
      current = new Set()
      sections.set(heading, current)
    } else if (current) {
      for (const [, name] of line.matchAll(/\{([a-z_]+)\}/g)) {
        current.add(name)
      }
    }
  }
  return sections
}

describe('PROMPT_SECTIONS', () => {
  it('lists every section of the shipped prompt, in order', () => {
    expect(PROMPT_SECTIONS.map((section) => section.heading)).toEqual([
      ...shippedSections().keys(),
    ])
  })

  it('lists every variable the shipped prompt uses under its section', () => {
    for (const [heading, used] of shippedSections()) {
      const guide = PROMPT_SECTIONS.find((section) => section.heading === heading)
      const listed = guide?.variables.map((variable) => variable.name) ?? []
      expect(listed, heading).toEqual(expect.arrayContaining([...used]))
    }
  })

  it('describes every section and variable', () => {
    for (const section of PROMPT_SECTIONS) {
      expect(section.purpose, section.heading).not.toBe('')
      for (const variable of section.variables) {
        expect(variable.description, `${section.heading} {${variable.name}}`).not.toBe('')
      }
    }
  })
})
