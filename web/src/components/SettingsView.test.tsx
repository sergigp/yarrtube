import { describe, expect, it } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
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
      screen.getAllByRole('link').map((link) => link.getAttribute('aria-label')),
    ).toEqual(['General', 'AI', 'Plex'])
  })

  it("shows each section's status under its name", async () => {
    mockApi({
      'GET /api/preferences': aPreferences({ ai_enabled: true, ai_provider: 'anthropic' }),
      'GET /api/plex': aPlexStatus(),
    })

    renderSettings('/settings/general')

    await waitFor(() =>
      expect(screen.getByRole('link', { name: 'AI' })).toHaveAccessibleDescription('On · Anthropic'),
    )
    await waitFor(() =>
      expect(screen.getByRole('link', { name: 'Plex' })).toHaveAccessibleDescription('Connected'),
    )
    expect(screen.getByRole('link', { name: 'General' })).toHaveAccessibleDescription(
      'Home and categories',
    )
  })

  it.each([
    ['/settings', '/settings/general', 'General'],
    ['/preferences', '/settings/general', 'General'],
    ['/ai', '/settings/ai', 'AI'],
  ])('opens %s at %s', async (route, expected, section) => {
    mockApi({ 'GET /api/preferences': aPreferences() })

    renderSettings(route)

    expect(await screen.findByRole('link', { name: section })).toHaveAttribute(
      'aria-current',
      'page',
    )
    expect(screen.getByRole('status', { name: 'Current route' })).toHaveTextContent(
      new RegExp(`^${expected}$`),
    )
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
