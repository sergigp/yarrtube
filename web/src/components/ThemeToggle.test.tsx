import { beforeEach, describe, expect, it } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { ThemeToggle } from './ThemeToggle'
import { aPreferences, mockApi, pendingForever, renderWithProviders } from '@/test/helpers'

describe('ThemeToggle', () => {
  beforeEach(() => {
    window.localStorage.clear()
  })

  it('switches to dark mode and saves it as a preference', async () => {
    const fetchMock = mockApi({
      'GET /api/preferences': aPreferences({ home_show_latest: false }),
      'PUT /api/preferences': aPreferences({ home_show_latest: false, theme: 'dark' }),
    })
    renderWithProviders(<ThemeToggle />)
    const user = userEvent.setup()

    await waitFor(() => expect(screen.getByRole('switch', { name: 'Dark mode' })).toBeEnabled())
    await user.click(screen.getByRole('switch', { name: 'Dark mode' }))

    expect(screen.getByRole('switch', { name: 'Dark mode' })).toBeChecked()
    expect(document.documentElement).toHaveClass('dark')
    expect(window.localStorage.getItem('yarrtube.theme')).toBe('dark')
    await waitFor(() =>
      expect(fetchMock).toHaveBeenCalledWith(
        '/api/preferences',
        expect.objectContaining({
          method: 'PUT',
          body: JSON.stringify(aPreferences({ home_show_latest: false, theme: 'dark' })),
        }),
      ),
    )
  })

  it('switches back to light mode', async () => {
    document.documentElement.classList.add('dark')
    mockApi({
      'GET /api/preferences': aPreferences({ theme: 'dark' }),
      'PUT /api/preferences': aPreferences({ theme: 'light' }),
    })
    renderWithProviders(<ThemeToggle />)
    const user = userEvent.setup()

    await waitFor(() => expect(screen.getByRole('switch', { name: 'Dark mode' })).toBeEnabled())
    await user.click(screen.getByRole('switch', { name: 'Dark mode' }))

    expect(screen.getByRole('switch', { name: 'Dark mode' })).not.toBeChecked()
    expect(document.documentElement).not.toHaveClass('dark')
  })

  it('switches back if the theme fails to save', async () => {
    mockApi({
      'GET /api/preferences': aPreferences(),
      'PUT /api/preferences': { status: 500, error: 'disk full' },
    })
    renderWithProviders(<ThemeToggle />)
    const user = userEvent.setup()

    await waitFor(() => expect(screen.getByRole('switch', { name: 'Dark mode' })).toBeEnabled())
    await user.click(screen.getByRole('switch', { name: 'Dark mode' }))

    await waitFor(() =>
      expect(screen.getByRole('switch', { name: 'Dark mode' })).not.toBeChecked(),
    )
    expect(document.documentElement).not.toHaveClass('dark')
  })

  it('is disabled until the preferences load', () => {
    mockApi({ 'GET /api/preferences': pendingForever() })

    renderWithProviders(<ThemeToggle />)

    expect(screen.getByRole('switch', { name: 'Dark mode' })).toBeDisabled()
  })
})
