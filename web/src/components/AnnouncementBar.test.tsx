import { beforeEach, describe, expect, it } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { AnnouncementBar } from './AnnouncementBar'
import { ANNOUNCEMENTS_URL } from '@/api/client'
import type { RouteResponse } from '@/test/helpers'
import { anAnnouncement, mockApi, renderWithProviders } from '@/test/helpers'

const DISMISSED_KEY = 'yarrtube.dismissedAnnouncements'

function renderBar(announcements: RouteResponse) {
  mockApi({ [`GET ${ANNOUNCEMENTS_URL}`]: announcements })
  return renderWithProviders(<AnnouncementBar />)
}

/** Renders the bar and waits until the announcements request has settled. */
async function renderBarAndSettle(announcements: RouteResponse) {
  const { queryClient } = renderBar(announcements)
  await waitFor(() => expect(queryClient.isFetching()).toBe(0))
}

describe('AnnouncementBar', () => {
  beforeEach(() => {
    window.localStorage.clear()
  })

  it('shows the newest unread announcement', async () => {
    renderBar([
      anAnnouncement({ id: 'b', text: 'Second' }),
      anAnnouncement({ id: 'a', text: 'First' }),
    ])

    expect(await screen.findByText('Second')).toBeInTheDocument()
    expect(screen.queryByText('First')).not.toBeInTheDocument()
  })

  it('renders inline links that open in a new tab', async () => {
    renderBar([anAnnouncement({ text: 'New release, see [docs](https://example.com/docs) now' })])

    const link = await screen.findByRole('link', { name: 'docs' })
    expect(link).toHaveAttribute('href', 'https://example.com/docs')
    expect(link).toHaveAttribute('target', '_blank')
    expect(link).toHaveAttribute('rel', 'noopener noreferrer')
    expect(screen.getByText(/New release, see/).parentElement).toHaveTextContent(
      'New release, see docs now',
    )
  })

  it('shows the next announcement after dismissing', async () => {
    const user = userEvent.setup()
    renderBar([
      anAnnouncement({ id: 'b', text: 'Second' }),
      anAnnouncement({ id: 'a', text: 'First' }),
    ])

    await user.click(await screen.findByRole('button', { name: 'Dismiss announcement' }))

    expect(screen.getByRole('region', { name: 'Announcement' })).toHaveTextContent('First')
    expect(screen.queryByText('Second')).not.toBeInTheDocument()
  })

  it('skips announcements dismissed earlier', async () => {
    window.localStorage.setItem(DISMISSED_KEY, JSON.stringify(['b']))
    renderBar([
      anAnnouncement({ id: 'b', text: 'Second' }),
      anAnnouncement({ id: 'a', text: 'First' }),
    ])

    expect(await screen.findByRole('region', { name: 'Announcement' })).toHaveTextContent('First')
  })

  it('renders nothing once every announcement is dismissed', async () => {
    window.localStorage.setItem(DISMISSED_KEY, JSON.stringify(['a', 'b']))

    await renderBarAndSettle([
      anAnnouncement({ id: 'b', text: 'Second' }),
      anAnnouncement({ id: 'a', text: 'First' }),
    ])

    expect(screen.queryByRole('region', { name: 'Announcement' })).not.toBeInTheDocument()
  })

  it('remembers dismissals across remounts', async () => {
    const user = userEvent.setup()
    const announcements = [
      anAnnouncement({ id: 'b', text: 'Second' }),
      anAnnouncement({ id: 'a', text: 'First' }),
    ]
    const { unmount } = renderBar(announcements)
    await user.click(await screen.findByRole('button', { name: 'Dismiss announcement' }))
    unmount()

    renderBar(announcements)

    expect(await screen.findByRole('region', { name: 'Announcement' })).toHaveTextContent('First')
  })

  it('renders nothing when the fetch fails', async () => {
    await renderBarAndSettle({ status: 500, error: 'boom' })

    expect(screen.queryByRole('region', { name: 'Announcement' })).not.toBeInTheDocument()
    expect(screen.queryByText(/boom/)).not.toBeInTheDocument()
  })

  it('renders nothing when the body is not an array', async () => {
    await renderBarAndSettle({ id: 'a', text: 'First' })

    expect(screen.queryByRole('region', { name: 'Announcement' })).not.toBeInTheDocument()
  })

  it('skips entries without a string id or text', async () => {
    renderBar([
      { text: 'No id' },
      { id: 'c', text: 7 },
      anAnnouncement({ id: 'a', text: 'First' }),
    ])

    expect(await screen.findByRole('region', { name: 'Announcement' })).toHaveTextContent('First')
  })

  it('wraps the whole text with its links inline, then the dismiss control', async () => {
    renderBar([
      anAnnouncement({ text: 'A long release note, see [docs](https://example.com/docs)' }),
    ])

    const region = await screen.findByRole('region', { name: 'Announcement' })
    expect(screen.getByText(/A long release note/).parentElement).not.toHaveClass(
      'max-sm:truncate',
    )
    expect(screen.getByRole('link', { name: 'docs' })).toBeVisible()
    expect(region.lastElementChild).toBe(
      screen.getByRole('button', { name: 'Dismiss announcement' }),
    )
  })
})
