import { describe, expect, it } from 'vitest'
import { channelPath, playlistPath } from './sourcePath'

describe('channelPath', () => {
  it('keeps a plain handle readable', () => {
    expect(channelPath('@somechannel')).toBe('/channels/@somechannel')
  })

  it('encodes a non-Latin handle', () => {
    expect(channelPath('@Сон')).toBe('/channels/@%D0%A1%D0%BE%D0%BD')
  })

  it('escapes a literal % so the router decodes back the stored id', () => {
    expect(decodeURIComponent(channelPath('@%D0%9D').slice('/channels/'.length))).toBe('@%D0%9D')
  })
})

describe('playlistPath', () => {
  it('builds the playlist route', () => {
    expect(playlistPath('PL123')).toBe('/playlists/PL123')
  })
})
