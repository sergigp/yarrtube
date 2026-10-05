import { describe, expect, it } from 'vitest'
import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { AppShell } from './App'
import { ANNOUNCEMENTS_URL } from '@/api/client'
import { mockApi, pendingForever, renderWithProviders } from '@/test/helpers'

function renderShell({ route = '/' } = {}) {
  mockApi({
    'GET /api/channels': [],
    'GET /api/playlists': [],
    'GET /api/videos/home': pendingForever(),
    'GET /api/tasks': pendingForever(),
    [`GET ${ANNOUNCEMENTS_URL}`]: [],
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

  it('goes home and closes the sidebar when the logo is clicked', async () => {
    renderShell({ route: '/tasks' })
    const user = userEvent.setup()
    await user.click(screen.getByRole('button', { name: 'Open menu' }))

    await user.click(screen.getByRole('link', { name: 'Yarrtube' }))

    expect(await screen.findByRole('heading', { name: 'Latest videos' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Open menu' })).toHaveAttribute(
      'aria-expanded',
      'false',
    )
  })

  it('closes the sidebar when the logo is clicked while already home', async () => {
    renderShell({ route: '/' })
    const user = userEvent.setup()
    await user.click(screen.getByRole('button', { name: 'Open menu' }))

    await user.click(screen.getByRole('link', { name: 'Yarrtube' }))

    expect(screen.getByRole('button', { name: 'Open menu' })).toHaveAttribute(
      'aria-expanded',
      'false',
    )
  })

  it('closes the sidebar when navigating from the settings menu', async () => {
    renderShell({ route: '/' })
    const user = userEvent.setup()
    await user.click(screen.getByRole('button', { name: 'Open menu' }))

    await user.click(screen.getByRole('button', { name: 'Settings' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Tasks' }))

    expect(await screen.findByText('Loading tasks…')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Open menu' })).toHaveAttribute(
      'aria-expanded',
      'false',
    )
  })
})
