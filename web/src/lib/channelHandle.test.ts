import { describe, expect, it } from 'vitest'
import { deriveChannelPathSegment } from './channelHandle'

describe('deriveChannelPathSegment', () => {
  it('returns an empty segment for empty or missing input', () => {
    expect(deriveChannelPathSegment('')).toBe('')
    expect(deriveChannelPathSegment('   ')).toBe('')
    expect(deriveChannelPathSegment(null)).toBe('')
    expect(deriveChannelPathSegment(undefined)).toBe('')
  })

  it('decodes a percent-encoded handle, bare or in a URL', () => {
    const encoded = '@%D0%9D%D0%B0%D1%83%D1%87%D0%BD%D1%8B%D0%B9%D0%A1%D0%BE%D0%BD'
    expect(deriveChannelPathSegment(encoded)).toBe('НаучныйСон')
    expect(deriveChannelPathSegment(`https://www.youtube.com/${encoded}/videos`)).toBe('НаучныйСон')
  })

  it('keeps a malformed percent sequence as entered', () => {
    expect(deriveChannelPathSegment('@bad%zz')).toBe('bad%zz')
  })

  it('strips a leading @ from a bare handle', () => {
    expect(deriveChannelPathSegment('@somechannel')).toBe('somechannel')
    expect(deriveChannelPathSegment('somechannel')).toBe('somechannel')
  })

  it('takes the handle segment of a channel URL', () => {
    expect(deriveChannelPathSegment('https://youtube.com/@name')).toBe('name')
    expect(deriveChannelPathSegment('https://www.youtube.com/@name/videos')).toBe('name')
    expect(deriveChannelPathSegment('http://youtube.com/@name/featured')).toBe('name')
  })

  it('ignores query strings and fragments in URLs', () => {
    expect(deriveChannelPathSegment('https://youtube.com/@name?si=abc')).toBe('name')
    expect(deriveChannelPathSegment('https://youtube.com/@name#about')).toBe('name')
  })

  it('returns an empty segment for a URL without a path', () => {
    expect(deriveChannelPathSegment('https://youtube.com')).toBe('')
    expect(deriveChannelPathSegment('https://youtube.com/')).toBe('')
  })
})
