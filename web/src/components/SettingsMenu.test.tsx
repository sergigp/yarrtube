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

    expect(items).toEqual(['Tasks'])
    expect(screen.getByRole('status', { name: 'Current route' })).toHaveTextContent(
      /^\/tasks$/,
    )
  })
})
