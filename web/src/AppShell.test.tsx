import { describe, expect, it } from 'vitest'
import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { AppShell } from './App'
import { mockApi, pendingForever, renderWithProviders } from '@/test/helpers'

function renderShell({ route = '/' } = {}) {
  mockApi({
    'GET /api/channels': [],
    'GET /api/playlists': [],
    'GET /api/videos/home': pendingForever(),
    'GET /api/tasks': pendingForever(),
  })
  return renderWithProviders(<AppShell />, { route })
}

describe('AppShell', () => {
  it('opens and closes the sidebar from the header toggle', async () => {
    renderShell()
    const user = userEvent.setup()

    await user.click(screen.getByRole('button', { name: 'Open menu' }))
    expect(screen.getByRole('button', { name: 'Close menu' })).toHaveAttribute(
      'aria-expanded',
      'true',
    )

    await user.click(screen.getByRole('button', { name: 'Close menu' }))
    expect(screen.getByRole('button', { name: 'Open menu' })).toHaveAttribute(
      'aria-expanded',
      'false',
    )
  })
})
