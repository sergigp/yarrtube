import { describe, expect, it } from 'vitest'
import { storageLocation } from './storageLocation'

describe('storageLocation', () => {
  it('shows the season folder of an episode file', () => {
    expect(storageLocation('channels/some-channel', 'Season 2026/S2026E031501 - Title.mp4')).toBe(
      'channels/some-channel/Season 2026',
    )
  })

  it('shows the storage path when the video has no file', () => {
    expect(storageLocation('channels/some-channel', null)).toBe('channels/some-channel')
  })
})
