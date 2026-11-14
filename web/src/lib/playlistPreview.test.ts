import { describe, expect, it } from 'vitest'
import { previewRows } from './playlistPreview'

describe('previewRows', () => {
  it('orders rows and drops the watched one when removing', () => {
    expect(previewRows('keep', 'newest_first')).toEqual([
      { label: 'Published today', watched: false },
      { label: 'Published last week', watched: true },
      { label: 'Published last month', watched: false },
    ])
    expect(previewRows('keep', 'oldest_first').map((row) => row.label)).toEqual([
      'Published last month',
      'Published last week',
      'Published today',
    ])
    expect(previewRows('drop', 'newest_first')).toEqual([
      { label: 'Published today', watched: false },
      { label: 'Published last month', watched: false },
    ])
  })
})
