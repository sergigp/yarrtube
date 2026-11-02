import { describe, expect, it } from 'vitest'
import { channelPath, playlistPath, youtubeChannelUrl, youtubePlaylistUrl } from './sourcePath'

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

describe('youtubeChannelUrl', () => {
  it('links the handle page', () => {
    expect(youtubeChannelUrl('@somechannel')).toBe('https://www.youtube.com/@somechannel')
  })

  it('encodes a non-Latin handle', () => {
    expect(youtubeChannelUrl('@Сон')).toBe('https://www.youtube.com/@%D0%A1%D0%BE%D0%BD')
  })
})

describe('youtubePlaylistUrl', () => {
  it('links the playlist page', () => {
    expect(youtubePlaylistUrl('PL123')).toBe('https://www.youtube.com/playlist?list=PL123')
  })
})
