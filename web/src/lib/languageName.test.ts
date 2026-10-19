import { describe, expect, it } from 'vitest'
import { languageName } from './languageName'

describe('languageName', () => {
  it('names a language in English', () => {
    expect(['en', 'es'].map(languageName)).toEqual(['English', 'Spanish'])
  })

  it('falls back to the code of an unknown or malformed language', () => {
    expect(['zz', 'not a code'].map(languageName)).toEqual(['zz', 'not a code'])
  })
})
