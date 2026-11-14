import { describe, expect, it } from 'vitest'
import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Routes } from 'react-router-dom'
import { settingsRoutes } from './settingsRoutes'
import { aPlexStatus, aPreferences, mockApi, renderWithProviders } from '@/test/helpers'
import { RouteProbe } from '@/test/RouteProbe'

describe('SettingsView', () => {
  it('switches sections from its nav and marks the open one', async () => {
    mockApi({
      'GET /api/preferences': aPreferences(),
      'GET /api/plex': aPlexStatus(),
    })
    renderSettings('/settings/general')
    const user = userEvent.setup()

    await user.click(await screen.findByRole('link', { name: 'Plex' }))

    expect(screen.getByRole('status', { name: 'Current route' })).toHaveTextContent(
      /^\/settings\/plex$/,
    )
    expect(screen.getByRole('link', { name: 'Plex' })).toHaveAttribute('aria-current', 'page')
    expect(screen.getByRole('link', { name: 'General' })).not.toHaveAttribute('aria-current')
    expect(
      screen.getAllByRole('link').map((link) => link.textContent),
    ).toEqual(['General', 'AI', 'Plex'])
  })
})

function renderSettings(route: string) {
  return renderWithProviders(
    <>
      <Routes>{settingsRoutes()}</Routes>
      <RouteProbe />
    </>,
    { route },
  )
}
