import { describe, expect, it } from 'vitest'
import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { SettingsMenu } from './SettingsMenu'
import { renderWithProviders } from '@/test/helpers'
import { RouteProbe } from '@/test/RouteProbe'

describe('SettingsMenu', () => {
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

    expect(items).toEqual(['Tasks', 'Preferences', 'Report a bug'])
    expect(screen.getByRole('status', { name: 'Current route' })).toHaveTextContent(
      /^\/tasks$/,
    )
  })

  it('offers Preferences', async () => {
    renderWithProviders(
      <>
        <SettingsMenu />
        <RouteProbe />
      </>,
    )
    const user = userEvent.setup()

    await user.click(screen.getByRole('button', { name: 'Settings' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Preferences' }))

    expect(screen.getByRole('status', { name: 'Current route' })).toHaveTextContent(
      /^\/preferences$/,
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
})
