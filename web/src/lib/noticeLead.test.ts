import { describe, expect, it } from 'vitest'
import { channelNoticeLead, playlistNoticeLead } from './noticeLead'

describe('playlistNoticeLead', () => {
  it('leaves the count out for an empty playlist', () => {
    expect(playlistNoticeLead(0, 'Mix')).toBe('Videos from “Mix” will be downloaded to')
  })

  it('singles out a one-video playlist', () => {
    expect(playlistNoticeLead(1, 'Mix')).toBe('The only video from “Mix” will be downloaded to')
  })

  it('states the count for larger playlists', () => {
    expect(playlistNoticeLead(12, 'Mix')).toBe('All 12 videos from “Mix” will be downloaded to')
  })
})

describe('channelNoticeLead', () => {
  it('leaves the count out when the limit field is empty or invalid', () => {
    const expected = 'Videos from “Chan” will be downloaded to'
    expect(channelNoticeLead('', 'Chan')).toBe(expected)
    expect(channelNoticeLead('  ', 'Chan')).toBe(expected)
    expect(channelNoticeLead('abc', 'Chan')).toBe(expected)
    expect(channelNoticeLead('2.5', 'Chan')).toBe(expected)
    expect(channelNoticeLead('0', 'Chan')).toBe(expected)
    expect(channelNoticeLead('1001', 'Chan')).toBe(expected)
  })

  it('singles out a limit of one', () => {
    expect(channelNoticeLead('1', 'Chan')).toBe(
      'The latest video from “Chan” will be downloaded to',
    )
  })

  it('states the limit when it is valid', () => {
    expect(channelNoticeLead('3', 'Chan')).toBe(
      'The latest 3 videos from “Chan” will be downloaded to',
    )
    expect(channelNoticeLead('1000', 'Chan')).toBe(
      'The latest 1000 videos from “Chan” will be downloaded to',
    )
  })
})
