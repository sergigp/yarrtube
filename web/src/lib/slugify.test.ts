import { describe, expect, it } from 'vitest'
import { slugify } from './slugify'

describe('slugify', () => {
  it('lowercases and joins words with single dashes', () => {
    expect(slugify('My Channel Name')).toBe('my-channel-name')
  })

  it('collapses runs of non-alphanumeric characters', () => {
    expect(slugify('Rock & Roll!!! (live)')).toBe('rock-roll-live')
  })

  it('trims leading and trailing dashes', () => {
    expect(slugify('  --Hello--  ')).toBe('hello')
  })

  it('keeps letters from non-Latin scripts', () => {
    expect(slugify('НаучныйСон')).toBe('научныйсон')
    expect(slugify('Научный Сон')).toBe('научный-сон')
    expect(slugify('東京 チャンネル')).toBe('東京-チャンネル')
  })

  it('keeps accented Latin letters', () => {
    expect(slugify('Café Tacvba')).toBe('café-tacvba')
  })

  it('composes decomposed characters so the same name gives the same slug', () => {
    expect(slugify('Сон\u0438\u0306')).toBe('сонй')
  })

  it('returns an empty slug for empty or missing input', () => {
    expect(slugify('')).toBe('')
    expect(slugify(null)).toBe('')
    expect(slugify(undefined)).toBe('')
    expect(slugify('!!!')).toBe('')
  })
})
