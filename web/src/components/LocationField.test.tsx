import { describe, expect, it, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { LocationField, type LocationValue } from './LocationField'
import { aChannel, aPlaylist, mockApi, type Routes } from '@/test/helpers'

const baseRoutes: Routes = {
  'GET /api/playlists': [],
  'GET /api/channels': [],
  'GET /api/directories': { root: '/videos', path: '', entries: [] },
  'GET /api/directories?path=playlists': { root: '/videos', path: 'playlists', entries: [] },
}

function lastReported(onChange: ReturnType<typeof vi.fn>): LocationValue {
  const lastCall = onChange.mock.calls[onChange.mock.calls.length - 1] as [LocationValue]
  return lastCall[0]
}

describe('LocationField', () => {
  it('derives the folder name from the name source and reports the destination', async () => {
    mockApi(baseRoutes)
    const onChange = vi.fn()

    render(<LocationField mode="playlist" nameSource="My Mix" onChange={onChange} />)

    expect(screen.getByLabelText('Folder name')).toHaveValue('my-mix')
    await waitFor(() =>
      expect(lastReported(onChange)).toEqual({
        path: 'playlists/my-mix',
        destination: '/videos/playlists/my-mix',
        valid: true,
        occupiedBy: undefined,
      }),
    )
  })

  it('keeps a hand-edited folder name when the name source changes', async () => {
    mockApi(baseRoutes)
    const onChange = vi.fn()
    const { rerender } = render(
      <LocationField mode="playlist" nameSource="My Mix" onChange={onChange} />,
    )
    const user = userEvent.setup()

    const folderName = screen.getByLabelText('Folder name')
    await user.clear(folderName)
    await user.type(folderName, 'custom-folder')
    rerender(<LocationField mode="playlist" nameSource="Another Name" onChange={onChange} />)

    expect(screen.getByLabelText('Folder name')).toHaveValue('custom-folder')
  })

  it('requires a folder name', async () => {
    mockApi(baseRoutes)
    const onChange = vi.fn()
    render(<LocationField mode="playlist" nameSource="My Mix" onChange={onChange} />)

    await userEvent.setup().clear(screen.getByLabelText('Folder name'))

    expect(screen.getByText('Folder name is required.')).toBeInTheDocument()
    await waitFor(() => expect(lastReported(onChange).valid).toBe(false))
  })

  it('reports the entry occupying the destination', async () => {
    mockApi({
      ...baseRoutes,
      'GET /api/playlists': [aPlaylist({ name: 'Existing Mix', path: 'playlists/my-mix' })],
      'GET /api/channels': [aChannel({ name: 'Some Channel', path: 'channels/some' })],
    })
    const onChange = vi.fn()

    render(<LocationField mode="playlist" nameSource="My Mix" onChange={onChange} />)

    await waitFor(() => {
      const reported = lastReported(onChange)
      expect(reported.occupiedBy).toBe('Existing Mix')
      expect(reported.valid).toBe(false)
    })
  })

  it('browses into subfolders and marks the ones in use', async () => {
    mockApi({
      ...baseRoutes,
      'GET /api/playlists': [aPlaylist({ name: 'Kids Mix', path: 'playlists/kids' })],
      'GET /api/directories?path=playlists': {
        root: '/videos',
        path: 'playlists',
        entries: [{ name: 'kids' }, { name: 'music' }],
      },
      'GET /api/directories?path=playlists%2Fmusic': {
        root: '/videos',
        path: 'playlists/music',
        entries: [],
      },
    })
    const onChange = vi.fn()
    render(<LocationField mode="playlist" nameSource="My Mix" onChange={onChange} />)
    const user = userEvent.setup()

    await user.click(screen.getByRole('button', { name: 'Parent folder' }))

    expect(await screen.findByText('kids')).toBeInTheDocument()
    expect(screen.getByText('in use by Kids Mix')).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: /music/ }))

    expect(await screen.findByText('No subfolders here.')).toBeInTheDocument()
    await waitFor(() =>
      expect(lastReported(onChange).path).toBe('playlists/music/my-mix'),
    )
  })

  it('stages a new folder that does not exist yet', async () => {
    mockApi(baseRoutes)
    const onChange = vi.fn()
    render(<LocationField mode="playlist" nameSource="My Mix" onChange={onChange} />)
    const user = userEvent.setup()

    await user.click(screen.getByRole('button', { name: 'Parent folder' }))
    await user.click(await screen.findByRole('button', { name: 'New folder' }))
    await user.type(screen.getByLabelText('New folder name'), 'fresh')
    await user.click(screen.getByRole('button', { name: 'Use folder' }))

    expect(
      await screen.findByText('Does not exist yet — created on the first download.'),
    ).toBeInTheDocument()
    await waitFor(() =>
      expect(lastReported(onChange)).toEqual({
        path: 'playlists/fresh/my-mix',
        destination: '/videos/playlists/fresh/my-mix',
        valid: true,
        occupiedBy: undefined,
      }),
    )
  })

  it('rejects a new folder name containing a slash', async () => {
    mockApi(baseRoutes)
    render(<LocationField mode="playlist" nameSource="My Mix" onChange={vi.fn()} />)
    const user = userEvent.setup()

    await user.click(screen.getByRole('button', { name: 'Parent folder' }))
    await user.click(await screen.findByRole('button', { name: 'New folder' }))
    await user.type(screen.getByLabelText('New folder name'), 'a/b')
    await user.click(screen.getByRole('button', { name: 'Use folder' }))

    expect(await screen.findByText('Folder name must not contain "/".')).toBeInTheDocument()
  })
})
