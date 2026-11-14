import { afterEach, describe, expect, it, vi } from 'vitest'
import { screen } from '@testing-library/react'
import { PlexSettingsView } from './PlexSettingsView'
import { aPlexStatus, aPreferences, mockApi, renderWithProviders } from '@/test/helpers'

describe('PlexSettingsView', () => {
  afterEach(() => {
    vi.useRealTimers()
  })

  it('explains how to set Plex up when it is not configured', async () => {
    mockApi({
      'GET /api/plex': aPlexStatus({ configured: false, connection: null, syncs: [] }),
      'GET /api/preferences': aPreferences(),
    })

    renderWithProviders(<PlexSettingsView />)

    expect(await screen.findByText(/Plex isn't configured/)).toHaveTextContent(
      "Plex isn't configured. Set YARRTUBE_PLEX_URL, YARRTUBE_PLEX_TOKEN and YARRTUBE_PLEX_VIDEOS_PATH to connect yarrtube to your Plex server.",
    )
    expect(screen.getByRole('link', { name: 'Plex guide' })).toHaveAttribute(
      'href',
      'https://github.com/sergigp/yarrtube/blob/main/doc/PLEX.md',
    )
    expect(screen.queryByRole('button', { name: /Sync now/ })).not.toBeInTheDocument()
    expect(screen.queryByRole('group', { name: 'Category playlists' })).not.toBeInTheDocument()
  })
})
