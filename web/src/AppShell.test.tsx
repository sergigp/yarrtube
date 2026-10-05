import { describe, expect, it } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { AppShell } from './App'
import { ANNOUNCEMENTS_URL } from '@/api/client'
import { queryKeys } from '@/api/queries'
import { mockApi, pendingForever, renderWithProviders, type Routes } from '@/test/helpers'

function renderShell({
  route = '/',
  version = pendingForever(),
}: { route?: string; version?: Routes[string] } = {}) {
  mockApi({
    'GET /api/channels': [],
    'GET /api/playlists': [],
    'GET /api/videos/home': pendingForever(),
    'GET /api/tasks': pendingForever(),
    'GET /api/version': version,
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

  it('shows the running version next to the logo', async () => {
    renderShell({ version: { version: 'v0.9.2' } })

    expect(await within(screen.getByRole('banner')).findByText('v0.9.2')).toBeInTheDocument()
  })

  it('stays on the current view when the version is clicked', async () => {
    renderShell({ route: '/tasks', version: { version: 'v0.9.2' } })
    const user = userEvent.setup()

    await user.click(await screen.findByText('v0.9.2'))

    expect(screen.getByText('Loading tasks…')).toBeInTheDocument()
  })

  it('shows no version while it is loading', () => {
    renderShell({ version: pendingForever() })

    expect(screen.getByRole('banner')).toHaveTextContent(/^$/)
  })

  it('shows no version and no error when the request fails', async () => {
    const { queryClient } = renderShell({ version: { status: 500, error: 'version unavailable' } })
    await waitFor(() => expect(queryClient.isFetching({ queryKey: queryKeys.version })).toBe(0))

    expect(screen.getByRole('banner')).toHaveTextContent(/^$/)
    expect(screen.queryByText(/version unavailable/)).not.toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'Yarrtube' })).toBeInTheDocument()
  })
})
