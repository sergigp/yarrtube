import { beforeEach, describe, expect, it } from 'vitest'
import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { SettingsMenu } from './SettingsMenu'
import { aTask, mockApi, renderWithProviders } from '@/test/helpers'
import { RouteProbe } from '@/test/RouteProbe'

describe('SettingsMenu', () => {
  beforeEach(() => {
    mockApi({ 'GET /api/tasks': [] })
  })

  it('offers Tasks', async () => {
    renderWithProviders(
      <>
        <SettingsMenu />
        <RouteProbe />
      </>,
    )
    const user = userEvent.setup()

    await user.click(screen.getByRole('button', { name: 'Settings' }))
    const items = (await screen.findAllByRole('menuitem')).map((item) => item.textContent)
    await user.click(screen.getByRole('menuitem', { name: 'Tasks' }))

    expect(items).toEqual(['Tasks', 'Settings', 'Report a bug'])
    expect(screen.getByRole('status', { name: 'Current route' })).toHaveTextContent(
      /^\/tasks$/,
    )
  })

  it('offers Settings', async () => {
    renderWithProviders(
      <>
        <SettingsMenu />
        <RouteProbe />
      </>,
    )
    const user = userEvent.setup()

    await user.click(screen.getByRole('button', { name: 'Settings' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Settings' }))

    expect(screen.getByRole('status', { name: 'Current route' })).toHaveTextContent(
      /^\/settings\/general$/,
    )
  })

  it('links Report a bug to a new GitHub issue in a new tab', async () => {
    renderWithProviders(<SettingsMenu />)
    const user = userEvent.setup()

    await user.click(screen.getByRole('button', { name: 'Settings' }))
    const item = await screen.findByRole('menuitem', { name: 'Report a bug' })

    expect(item).toHaveAttribute('href', 'https://github.com/sergigp/yarrtube/issues/new')
    expect(item).toHaveAttribute('target', '_blank')
  })

  it('shows how many tasks are running next to Tasks', async () => {
    mockApi({
      'GET /api/tasks': [
        aTask({ status: 'running' }),
        aTask({ status: 'running' }),
        aTask({ status: 'pending' }),
      ],
    })
    renderWithProviders(<SettingsMenu />)
    const user = userEvent.setup()

    await user.click(screen.getByRole('button', { name: 'Settings' }))

    expect(await screen.findByText('2 running')).toBeInTheDocument()
  })

  it('shows no running count when no task is running', async () => {
    mockApi({ 'GET /api/tasks': [aTask({ status: 'pending' })] })
    renderWithProviders(<SettingsMenu />)
    const user = userEvent.setup()

    await user.click(screen.getByRole('button', { name: 'Settings' }))

    expect(await screen.findByRole('menuitem', { name: 'Tasks' })).toBeInTheDocument()
    expect(screen.queryByText(/running/)).not.toBeInTheDocument()
  })
})
