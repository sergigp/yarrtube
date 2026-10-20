import { describe, expect, it } from 'vitest'
import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { SettingsMenu } from './SettingsMenu'
import { renderWithProviders, RouteProbe } from '@/test/helpers'

describe('SettingsMenu', () => {
  it('offers Categories', async () => {
    renderWithProviders(
      <>
        <SettingsMenu />
        <RouteProbe />
      </>,
    )
    const user = userEvent.setup()

    await user.click(screen.getByRole('button', { name: 'Settings' }))
    const items = (await screen.findAllByRole('menuitem')).map((item) => item.textContent)
    await user.click(screen.getByRole('menuitem', { name: 'Categories' }))

    expect(items).toEqual(['Categories', 'Tasks'])
    expect(screen.getByRole('status', { name: 'Current route' })).toHaveTextContent(
      /^\/categories$/,
    )
  })
})
