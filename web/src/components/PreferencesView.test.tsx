import { describe, expect, it } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
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

  it('saves every preference with the toggled one changed', async () => {
    const fetchMock = mockApi({
      'GET /api/preferences': aPreferences(),
      'PUT /api/preferences': aPreferences({ home_show_quick_watches: false }),
    })
    renderWithProviders(<PreferencesView />)
    const user = userEvent.setup()

    await user.click(await screen.findByRole('switch', { name: 'Show quick watches' }))

    await waitFor(() =>
      expect(screen.getByRole('switch', { name: 'Show quick watches' })).not.toBeChecked(),
    )
    expect(
      fetchMock.mock.calls
        .filter(([, init]) => (init as RequestInit | undefined)?.method === 'PUT')
        .map(([url, init]) => [url, JSON.parse((init as RequestInit).body as string)]),
    ).toEqual([['/api/preferences', aPreferences({ home_show_quick_watches: false })]])
  })

  it('reports a failed save and keeps the saved value', async () => {
    mockApi({
      'GET /api/preferences': aPreferences(),
      'PUT /api/preferences': { status: 500, error: 'disk full' },
    })
    renderWithProviders(<PreferencesView />)
    const user = userEvent.setup()

    await user.click(await screen.findByRole('switch', { name: 'Show quick watches' }))

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Failed to save preferences: disk full',
    )
    expect(screen.getByRole('switch', { name: 'Show quick watches' })).toBeChecked()
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
