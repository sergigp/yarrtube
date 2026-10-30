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

  it('shows a switch per preference with its current value', async () => {
    mockApi({ 'GET /api/preferences': aPreferences({ home_show_latest: false }) })

    renderWithProviders(<PreferencesView />)

    const categories = await screen.findByRole('group', { name: 'Categories' })
    const home = screen.getByRole('group', { name: 'Home' })
    expect(switchStates(categories)).toEqual([['Enable categories', true]])
    expect(switchStates(home)).toEqual([
      ['Show continue watching', true],
      ['Show categories', true],
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

  it('disables the home category switch while categories are off', async () => {
    mockApi({
      'GET /api/preferences': aPreferences({
        categories_enabled: false,
        home_show_categories: false,
      }),
    })

    renderWithProviders(<PreferencesView />)

    const showCategories = await screen.findByRole('switch', { name: 'Show categories' })
    expect(showCategories).toBeDisabled()
    expect(showCategories).not.toBeChecked()
    expect(screen.getByRole('switch', { name: 'Show latest videos' })).toBeEnabled()
  })

  it('explains a preference on hover', async () => {
    mockApi({ 'GET /api/preferences': aPreferences() })
    renderWithProviders(<PreferencesView />)

    await userEvent
      .setup()
      .hover(await screen.findByRole('button', { name: 'About Enable categories' }))

    expect(await screen.findByRole('tooltip')).toHaveTextContent(
      'Use categories across Yarrtube: the sidebar section, category pages, the Categories field when adding or editing a channel or playlist, and category sections on home. Turning it off only hides them: your categories and their videos are kept.',
    )
  })

  it('explains a preference on click, as a tap would', async () => {
    mockApi({ 'GET /api/preferences': aPreferences() })
    renderWithProviders(<PreferencesView />)
    const user = userEvent.setup({ skipHover: true })

    await user.click(await screen.findByRole('button', { name: 'About Show quick watches' }))

    expect(await screen.findByRole('tooltip')).toHaveTextContent(
      'Unwatched videos shorter than 15 minutes.',
    )
  })

  it('says how home sections share their videos', async () => {
    mockApi({ 'GET /api/preferences': aPreferences() })

    renderWithProviders(<PreferencesView />)

    expect(
      within(await screen.findByRole('group', { name: 'Home' })).getByText(
        'Sections show in this order, and a video shows in one section only: turning a section off can move its videos to a section below.',
      ),
    ).toBeInTheDocument()
  })

  it('says "Saved" after a save, for 2 seconds', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true })
    mockApi({
      'GET /api/preferences': aPreferences(),
      'PUT /api/preferences': aPreferences({ home_show_quick_watches: false }),
    })
    renderWithProviders(<PreferencesView />)
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime })

    await user.click(await screen.findByRole('switch', { name: 'Show quick watches' }))

    await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('Saved'))
    await act(() => vi.advanceTimersByTimeAsync(2000))
    expect(screen.getByRole('status')).toHaveTextContent('')
  })

  it('does not say "Saved" when the save fails', async () => {
    mockApi({
      'GET /api/preferences': aPreferences(),
      'PUT /api/preferences': { status: 500, error: 'disk full' },
    })
    renderWithProviders(<PreferencesView />)
    const user = userEvent.setup()

    await user.click(await screen.findByRole('switch', { name: 'Show quick watches' }))

    expect(await screen.findByRole('alert')).toHaveTextContent('disk full')
    expect(screen.queryByText('Saved')).not.toBeInTheDocument()
  })

  it('keeps a second toggle from undoing one still being saved', async () => {
    const firstSave = heldUntilReleased(aPreferences({ home_show_quick_watches: false }))
    const fetchMock = mockApi({
      'GET /api/preferences': aPreferences(),
      'PUT /api/preferences': () =>
        fetchMock.mock.calls.filter(([, init]) => init?.method === 'PUT').length === 1
          ? (firstSave.route as () => Promise<unknown>)()
          : aPreferences({ home_show_quick_watches: false, home_show_latest: false }),
    })
    renderWithProviders(<PreferencesView />)
    const user = userEvent.setup()

    await user.click(await screen.findByRole('switch', { name: 'Show quick watches' }))
    await waitFor(() => expect(firstSave.route).toHaveBeenCalled())
    expect(screen.getByRole('switch', { name: 'Show latest videos' })).toBeDisabled()
    firstSave.release()
    await waitFor(() =>
      expect(screen.getByRole('switch', { name: 'Show latest videos' })).toBeEnabled(),
    )
    await user.click(screen.getByRole('switch', { name: 'Show latest videos' }))

    await waitFor(() =>
      expect(screen.getByRole('switch', { name: 'Show latest videos' })).not.toBeChecked(),
    )
    expect(
      fetchMock.mock.calls
        .filter(([, init]) => (init as RequestInit | undefined)?.method === 'PUT')
        .map(([, init]) => JSON.parse((init as RequestInit).body as string)),
    ).toEqual([
      aPreferences({ home_show_quick_watches: false }),
      aPreferences({ home_show_quick_watches: false, home_show_latest: false }),
    ])
  })

  it('reloads cached home videos as soon as a preference is saved', async () => {
    const emptyHome = { continue_watching: [], categories: [], quick_watches: [], latest: [] }
    const fetchMock = mockApi({
      'GET /api/preferences': aPreferences(),
      'PUT /api/preferences': aPreferences({ home_show_quick_watches: false }),
      'GET /api/videos/home': emptyHome,
    })
    const { queryClient } = renderWithProviders(<PreferencesView />)
    // Home was visited earlier: its videos are cached, but no view shows them now.
    await queryClient.prefetchQuery({ queryKey: queryKeys.recentVideos, queryFn: fetchHomeVideos })
    const homeRequests = () => fetchMock.mock.calls.filter(([url]) => url === '/api/videos/home')

    await userEvent
      .setup()
      .click(await screen.findByRole('switch', { name: 'Show quick watches' }))

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
