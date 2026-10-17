import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { EditPlaylistDialog } from './EditPlaylistDialog'
import { aPlaylist } from '@/test/helpers'
import type { PlaylistListItem } from '@/api/types'

function renderDialog(
  playlist: PlaylistListItem,
  onSave: (changes: object) => Promise<void> = vi.fn(async () => {}),
) {
  const onOpenChange = vi.fn()
  render(<EditPlaylistDialog playlist={playlist} open onOpenChange={onOpenChange} onSave={onSave} />)
  return { onOpenChange, onSave }
}

describe('EditPlaylistDialog', () => {
  it("opens prefilled with the playlist's interval", () => {
    renderDialog(aPlaylist({ name: 'Lofi beats', reconcile_interval_minutes: 180 }))

    expect(screen.getByRole('dialog', { name: 'Edit Lofi beats settings' })).toBeInTheDocument()
    expect(screen.getByLabelText('Check for new videos every')).toHaveValue(3)
    expect(screen.getByRole('combobox', { name: 'Interval unit' })).toHaveTextContent('hours')
  })
})
