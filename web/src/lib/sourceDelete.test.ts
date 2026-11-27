import { describe, expect, it } from 'vitest'
import { deleteSourceDescription } from './sourceDelete'

describe('deleteSourceDescription', () => {
  it('says nothing about favorites without any, and counts them otherwise', () => {
    expect([
      deleteSourceDescription('channel', 0),
      deleteSourceDescription('playlist', 1),
      deleteSourceDescription('channel', 2),
    ]).toEqual([
      'This removes the channel from tracking.',
      'This removes the playlist from tracking, along with its video records and downloaded files. Its favorite video will be deleted too.',
      'This removes the channel from tracking. Its 2 favorite videos will be deleted too.',
    ])
  })
})
