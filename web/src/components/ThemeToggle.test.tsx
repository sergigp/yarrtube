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
      'PATCH /api/preferences': aPreferences({ home_show_latest: false, theme: 'dark' }),
    })
    renderWithProviders(<ThemeToggle />)
    const user = userEvent.setup()

    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Switch to dark theme' })).toBeEnabled(),
    )
    await user.click(screen.getByRole('button', { name: 'Switch to dark theme' }))

    expect(screen.getByRole('button', { name: 'Switch to light theme' })).toBeInTheDocument()
    expect(document.documentElement).toHaveClass('dark')
    expect(window.localStorage.getItem('yarrtube.theme')).toBe('dark')
    await waitFor(() =>
      expect(fetchMock).toHaveBeenCalledWith(
        '/api/preferences',
        expect.objectContaining({
          method: 'PATCH',
          body: JSON.stringify({ theme: 'dark' }),
        }),
      ),
    )
  })

  it('switches back to light mode', async () => {
    document.documentElement.classList.add('dark')
    mockApi({
      'GET /api/preferences': aPreferences({ theme: 'dark' }),
      'PATCH /api/preferences': aPreferences({ theme: 'light' }),
    })
    renderWithProviders(<ThemeToggle />)
    const user = userEvent.setup()

    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Switch to light theme' })).toBeEnabled(),
    )
    await user.click(screen.getByRole('button', { name: 'Switch to light theme' }))

    expect(screen.getByRole('button', { name: 'Switch to dark theme' })).toBeInTheDocument()
    expect(document.documentElement).not.toHaveClass('dark')
  })

  it('switches back if the theme fails to save', async () => {
    mockApi({
      'GET /api/preferences': aPreferences(),
      'PATCH /api/preferences': { status: 500, error: 'disk full' },
    })
    renderWithProviders(<ThemeToggle />)
    const user = userEvent.setup()

    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Switch to dark theme' })).toBeEnabled(),
    )
    await user.click(screen.getByRole('button', { name: 'Switch to dark theme' }))

    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Switch to dark theme' })).toBeInTheDocument(),
    )
    expect(document.documentElement).not.toHaveClass('dark')
  })

  it('is disabled until the preferences load', () => {
    mockApi({ 'GET /api/preferences': pendingForever() })

    renderWithProviders(<ThemeToggle />)

    expect(screen.getByRole('button', { name: 'Switch to dark theme' })).toBeDisabled()
  })
})
