import { afterEach, describe, expect, it, vi } from 'vitest'
import { act, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { PreferencesView } from './PreferencesView'
import { fetchHomeVideos } from '@/api/client'
import { queryKeys } from '@/api/queries'
import { aPreferences, heldUntilReleased, mockApi, renderWithProviders } from '@/test/helpers'

describe('PreferencesView', () => {
  afterEach(() => {
    vi.useRealTimers()
  })

  it('shows the renamed switches with their descriptions', async () => {
    mockApi({ 'GET /api/preferences': aPreferences({ home_show_latest: false }) })

    renderWithProviders(<PreferencesView />)

    const categories = await screen.findByRole('group', { name: 'Categories' })
    const home = screen.getByRole('group', { name: 'Home sections' })
    expect(switchStates(categories)).toEqual([['Categories', true]])
    expect(switchStates(home)).toEqual([
      ['Continue watching', true],
      ['More about…', true],
      ['Quick watches', true],
      ['Latest videos', false],
    ])
    expect(screen.getByRole('switch', { name: 'Categories' })).toHaveAccessibleDescription(
      'Group channels and playlists by topic and browse by category. Turning this off hides categories but keeps them and their videos.',
    )
    expect(screen.getByRole('switch', { name: 'Continue watching' })).toHaveAccessibleDescription(
      "Videos you started in the last 7 days and haven't finished.",
    )
    expect(screen.getByRole('switch', { name: 'More about…' })).toHaveAccessibleDescription(
      'One section for each of up to 3 categories with unwatched videos.',
    )
    expect(screen.getByRole('switch', { name: 'Quick watches' })).toHaveAccessibleDescription(
      'Unwatched videos under 15 minutes.',
    )
    expect(screen.getByRole('switch', { name: 'Latest videos' })).toHaveAccessibleDescription(
      "Your newest downloads that aren't in a section above.",
    )
    expect(
      within(home).getByText(
        'Shown top to bottom in this order. A video appears in one section only, so turning a section off can move its videos to the next one.',
      ),
    ).toBeInTheDocument()
    expect(screen.getByText('Changes save as you make them.')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /^About / })).not.toBeInTheDocument()
  })

  it('links categories to where they are also used', async () => {
    mockApi({ 'GET /api/preferences': aPreferences() })

    renderWithProviders(<PreferencesView />)

    const categories = await screen.findByRole('group', { name: 'Categories' })
    expect(within(categories).getByText('Also used by')).toBeInTheDocument()
    expect(within(categories).getByRole('link', { name: 'AI categorization' })).toHaveAttribute(
      'href',
      '/settings/ai',
    )
    expect(
      within(categories).getByRole('link', { name: 'Plex category playlists' }),
    ).toHaveAttribute('href', '/settings/plex')
    expect(within(categories).getByRole('link', { name: 'Home: More about' })).toHaveAttribute(
      'href',
      '#home-sections',
    )
  })

  it('saves only the toggled preference', async () => {
    const fetchMock = mockApi({
      'GET /api/preferences': aPreferences(),
      'PATCH /api/preferences': aPreferences({ home_show_quick_watches: false }),
    })
    renderWithProviders(<PreferencesView />)
    const user = userEvent.setup()

    await user.click(await screen.findByRole('switch', { name: 'Quick watches' }))

    await waitFor(() =>
      expect(screen.getByRole('switch', { name: 'Quick watches' })).not.toBeChecked(),
    )
    expect(
      fetchMock.mock.calls
        .filter(([, init]) => (init as RequestInit | undefined)?.method === 'PATCH')
        .map(([url, init]) => [url, JSON.parse((init as RequestInit).body as string)]),
    ).toEqual([['/api/preferences', { home_show_quick_watches: false }]])
  })

  it('reports a failed save and keeps the saved value', async () => {
    mockApi({
      'GET /api/preferences': aPreferences(),
      'PATCH /api/preferences': { status: 500, error: 'disk full' },
    })
    renderWithProviders(<PreferencesView />)
    const user = userEvent.setup()

    await user.click(await screen.findByRole('switch', { name: 'Quick watches' }))

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Failed to save preferences: disk full',
    )
    expect(screen.getByRole('switch', { name: 'Quick watches' })).toBeChecked()
  })

  it('disables the home category switch while categories are off', async () => {
    mockApi({
      'GET /api/preferences': aPreferences({
        categories_enabled: false,
        home_show_categories: false,
      }),
    })

    renderWithProviders(<PreferencesView />)

    const showCategories = await screen.findByRole('switch', { name: 'More about…' })
    expect(showCategories).toBeDisabled()
    expect(showCategories).not.toBeChecked()
    expect(screen.getByRole('switch', { name: 'Latest videos' })).toBeEnabled()
  })

  it('is headed General', async () => {
    mockApi({ 'GET /api/preferences': aPreferences() })

    renderWithProviders(<PreferencesView />)

    expect(await screen.findByRole('heading', { level: 1 })).toHaveTextContent('General')
  })

  it('says "Saved" after a save, for 2 seconds', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true })
    mockApi({
      'GET /api/preferences': aPreferences(),
      'PATCH /api/preferences': aPreferences({ home_show_quick_watches: false }),
    })
    renderWithProviders(<PreferencesView />)
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime })

    await user.click(await screen.findByRole('switch', { name: 'Quick watches' }))

    await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('Saved'))
    await act(() => vi.advanceTimersByTimeAsync(2000))
    expect(screen.getByRole('status')).toHaveTextContent('')
  })

  it('does not say "Saved" when the save fails', async () => {
    mockApi({
      'GET /api/preferences': aPreferences(),
      'PATCH /api/preferences': { status: 500, error: 'disk full' },
    })
    renderWithProviders(<PreferencesView />)
    const user = userEvent.setup()

    await user.click(await screen.findByRole('switch', { name: 'Quick watches' }))

    expect(await screen.findByRole('alert')).toHaveTextContent('disk full')
    expect(screen.queryByText('Saved')).not.toBeInTheDocument()
  })

  it('saves one toggle at a time', async () => {
    const firstSave = heldUntilReleased(aPreferences({ home_show_quick_watches: false }))
    const fetchMock = mockApi({
      'GET /api/preferences': aPreferences(),
      'PATCH /api/preferences': () =>
        fetchMock.mock.calls.filter(([, init]) => init?.method === 'PATCH').length === 1
          ? (firstSave.route as () => Promise<unknown>)()
          : aPreferences({ home_show_quick_watches: false, home_show_latest: false }),
    })
    renderWithProviders(<PreferencesView />)
    const user = userEvent.setup()

    await user.click(await screen.findByRole('switch', { name: 'Quick watches' }))
    await waitFor(() => expect(firstSave.route).toHaveBeenCalled())
    expect(screen.getByRole('switch', { name: 'Latest videos' })).toBeDisabled()
    firstSave.release()
    await waitFor(() =>
      expect(screen.getByRole('switch', { name: 'Latest videos' })).toBeEnabled(),
    )
    await user.click(screen.getByRole('switch', { name: 'Latest videos' }))

    await waitFor(() =>
      expect(screen.getByRole('switch', { name: 'Latest videos' })).not.toBeChecked(),
    )
    expect(
      fetchMock.mock.calls
        .filter(([, init]) => (init as RequestInit | undefined)?.method === 'PATCH')
        .map(([, init]) => JSON.parse((init as RequestInit).body as string)),
    ).toEqual([{ home_show_quick_watches: false }, { home_show_latest: false }])
  })

  it('reloads cached home videos as soon as a preference is saved', async () => {
    const emptyHome = { continue_watching: [], categories: [], quick_watches: [], latest: [] }
    const fetchMock = mockApi({
      'GET /api/preferences': aPreferences(),
      'PATCH /api/preferences': aPreferences({ home_show_quick_watches: false }),
      'GET /api/videos/home': emptyHome,
    })
    const { queryClient } = renderWithProviders(<PreferencesView />)
    // Home was visited earlier: its videos are cached, but no view shows them now.
    await queryClient.prefetchQuery({ queryKey: queryKeys.recentVideos, queryFn: fetchHomeVideos })
    const homeRequests = () => fetchMock.mock.calls.filter(([url]) => url === '/api/videos/home')

    await userEvent
      .setup()
      .click(await screen.findByRole('switch', { name: 'Quick watches' }))

    await waitFor(() => expect(homeRequests()).toHaveLength(2))
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
