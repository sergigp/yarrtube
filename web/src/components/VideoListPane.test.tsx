import { afterEach, describe, expect, it, vi } from 'vitest'
import { screen } from '@testing-library/react'
import { VideoListPane } from './VideoListPane'
import { aVideo, mockApi, renderWithProviders } from '@/test/helpers'
import type { Video } from '@/api/types'

function renderPane(videos: Video[] | undefined) {
  mockApi({})
  renderWithProviders(
    <VideoListPane
      noun="channel"
      basePathOf={() => 'channels/chan'}
      sourceNameOf={() => 'My Channel'}
      videos={videos}
      error={null}
      selectedVideoId={null}
      onSelect={() => {}}
    />,
  )
}

describe('VideoListPane', () => {
  afterEach(() => {
    vi.useRealTimers()
  })

  it('shows how long ago each video was published below its title', () => {
    vi.useFakeTimers({ toFake: ['Date'] })
    vi.setSystemTime(new Date('2026-10-30T12:00:00Z'))

    renderPane([aVideo({ title: 'Fresh Video', published_at: '2026-10-27T09:00:00Z' })])

    const row = screen.getByRole('button', { name: /^(?!Actions).*Fresh Video/ })
    expect(row).toHaveTextContent(/Fresh Video3d ago/)
  })

  it('says why a video that is not downloaded has no file', () => {
    renderPane([
      aVideo({ title: 'Pending', status: 'PENDING', filename: null }),
      aVideo({ title: 'Retrying', status: 'RETRYING', filename: null }),
      aVideo({ title: 'Failed', status: 'FAILED', filename: null }),
      aVideo({ title: 'Unavailable', status: 'UNAVAILABLE', filename: null }),
    ])

    expect(screen.getAllByRole('img').map((icon) => icon.getAttribute('aria-label'))).toEqual([
      'This video is pending.',
      'This video failed to download and will be retried.',
      'This video failed to download.',
      'This video is unavailable on YouTube.',
    ])
  })

  it('shows no date for a video without a publish date', () => {
    renderPane([aVideo({ title: 'Undated', published_at: null })])

    expect(screen.getByRole('button', { name: /^(?!Actions).*Undated/ })).not.toHaveTextContent(
      /ago|just now/,
    )
  })

  it('shows placeholder rows, marked busy, while the videos load', () => {
    renderPane(undefined)

    const status = screen.getByRole('status')
    expect(status).toHaveTextContent('Loading…')
    expect(status.closest('[aria-busy="true"]')).not.toBeNull()
    expect(screen.queryByText('Loading videos…')).not.toBeInTheDocument()
  })
})
