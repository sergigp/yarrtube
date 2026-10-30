import { describe, expect, it } from 'vitest'
import { screen, within } from '@testing-library/react'
import { PreferencesView } from './PreferencesView'
import { aPreferences, mockApi, renderWithProviders } from '@/test/helpers'

describe('PreferencesView', () => {
  it('shows a switch per preference with its current value', async () => {
    mockApi({ 'GET /api/preferences': aPreferences({ home_show_latest: false }) })

    renderWithProviders(<PreferencesView />)

    const categories = await screen.findByRole('group', { name: 'Categories' })
    const home = screen.getByRole('group', { name: 'Home' })
    expect(switchStates(categories)).toEqual([['Enable categories', true]])
    expect(switchStates(home)).toEqual([
      ['Show continue watching', true],
      ['Show categories', true],
      ['Show category colors', true],
      ['Show quick watches', true],
      ['Show latest videos', false],
    ])
  })
})

function switchStates(group: HTMLElement): [string, boolean][] {
  return within(group)
    .getAllByRole('switch')
    .map((element) => [
      element.getAttribute('aria-label') ?? labelOf(element),
      element.getAttribute('aria-checked') === 'true',
    ])
}

function labelOf(element: HTMLElement): string {
  const id = element.getAttribute('id')
  return document.querySelector(`label[for="${id}"]`)?.textContent ?? ''
}
