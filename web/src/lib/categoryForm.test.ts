import { describe, expect, it } from 'vitest'
import { changedDetails } from './categoryForm'

const base = { name: 'Chess', color: '#8b5cf6', prompt: 'Chess games' }

describe('changedDetails', () => {
  it('is empty when nothing changed', () => {
    expect(changedDetails(base, base, { withPrompt: true })).toEqual({})
  })

  it('has only what differs', () => {
    expect(changedDetails(base, { ...base, color: '#14b8a6' }, { withPrompt: true })).toEqual({
      color: '#14b8a6',
    })
    expect(changedDetails(base, { ...base, name: 'Board games' }, { withPrompt: true })).toEqual({
      name: 'Board games',
    })
    expect(changedDetails(base, { ...base, prompt: 'Openings' }, { withPrompt: true })).toEqual({
      ai_prompt: 'Openings',
    })
  })

  it('ignores surrounding spaces', () => {
    const fields = { ...base, name: ' Chess ', prompt: ' Chess games\n' }

    expect(changedDetails(base, fields, { withPrompt: true })).toEqual({})
  })

  it('removes an emptied prompt', () => {
    expect(changedDetails(base, { ...base, prompt: '  ' }, { withPrompt: true })).toEqual({
      ai_prompt: '',
    })
  })

  it('has no prompt without withPrompt', () => {
    expect(changedDetails(base, { ...base, prompt: 'Openings' }, { withPrompt: false })).toEqual({})
  })
})
