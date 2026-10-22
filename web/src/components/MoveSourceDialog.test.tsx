import { afterEach, describe, expect, it, vi } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { movePlaylist } from '@/api/client'
import { readRememberedParent } from '@/lib/saveLocations'
import { MoveSourceDialog } from './MoveSourceDialog'
import { aPlaylist, mockApi, renderWithProviders, type Routes } from '@/test/helpers'

const baseRoutes: Routes = {
  'GET /api/playlists': [aPlaylist({ id: 'PL1', name: 'Lofi beats', path: 'music/chill' })],
  'GET /api/channels': [],
  'GET /api/directories': { root: '/videos', path: '', entries: [] },
}

function renderDialog(routes: Routes = {}) {
  const fetchMock = mockApi({ ...baseRoutes, ...routes })
  const onClose = vi.fn()
  renderWithProviders(
    <MoveSourceDialog
      kind="playlist"
      name="Lofi beats"
      currentPath="music/chill"
      onMove={(path) => movePlaylist('PL1', path)}
      onClose={onClose}
    />,
  )
  return { fetchMock, onClose }
}

function sentRequests(fetchMock: ReturnType<typeof vi.fn>): [string, unknown][] {
  return fetchMock.mock.calls
    .filter(([, init]) => (init as RequestInit | undefined)?.method === 'POST')
    .map(([url, init]) => [url as string, JSON.parse((init as RequestInit).body as string)])
}

describe('MoveSourceDialog', () => {
  afterEach(() => {
    window.localStorage.clear()
  })

  it('opens prefilled with the current location', async () => {
    renderDialog()

    expect(screen.getByRole('dialog', { name: 'Move Lofi beats' })).toBeInTheDocument()
    expect(await screen.findByRole('radio', { name: 'music/' })).toBeChecked()
    expect(screen.getByLabelText('Folder name')).toHaveValue('chill')
    expect(await screen.findByText('/videos/music/chill')).toBeInTheDocument()
    expect(screen.getByTestId('destination-notice')).toHaveTextContent(
      'Videos and their files move to /videos/music/chill',
    )
  })

  it('disables moving while the destination is the current location', async () => {
    renderDialog()

    await screen.findByText('/videos/music/chill')

    expect(screen.getByRole('button', { name: 'Move' })).toBeDisabled()
  })

  it('moves to the new destination and closes', async () => {
    const { fetchMock, onClose } = renderDialog({
      'POST /api/playlists/PL1/move': aPlaylist({ id: 'PL1', path: 'music/lofi' }),
    })
    const user = userEvent.setup()
    const folderName = screen.getByLabelText('Folder name')

    await user.clear(folderName)
    await user.type(folderName, 'lofi')
    expect(await screen.findByText('/videos/music/lofi')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Move' }))

    await waitFor(() => expect(onClose).toHaveBeenCalled())
    expect(sentRequests(fetchMock)).toEqual([['/api/playlists/PL1/move', { path: 'music/lofi' }]])
    expect(readRememberedParent('playlist')).toBeNull()
  })

  it("keeps the dialog open and shows the server's error when the move is rejected", async () => {
    const { onClose } = renderDialog({
      'POST /api/playlists/PL1/move': {
        status: 409,
        error: 'playlist or channel still has work in progress, try again once its downloads finish',
      },
    })
    const user = userEvent.setup()
    const folderName = screen.getByLabelText('Folder name')

    await user.clear(folderName)
    await user.type(folderName, 'lofi')
    await user.click(screen.getByRole('button', { name: 'Move' }))

    expect(
      await screen.findByText(
        'playlist or channel still has work in progress, try again once its downloads finish',
      ),
    ).toBeInTheDocument()
    expect(onClose).not.toHaveBeenCalled()
  })

  it('says a new folder is created by the move', async () => {
    renderDialog({
      'GET /api/directories?path=music': { root: '/videos', path: 'music', entries: [{ name: 'chill' }] },
    })
    const user = userEvent.setup()

    await user.click(await screen.findByRole('button', { name: 'Choose another folder…' }))
    await user.click(await screen.findByRole('button', { name: 'New folder' }))
    await user.type(screen.getByLabelText('New folder name'), 'archive')
    await user.click(screen.getByRole('button', { name: 'Use folder' }))

    expect(await screen.findByText('Does not exist yet — created by the move.')).toBeInTheDocument()
  })

  it.each(['.', '..'])('refuses to move into a folder named "%s"', async (name) => {
    renderDialog()
    const user = userEvent.setup()
    const folderName = screen.getByLabelText('Folder name')

    await user.clear(folderName)
    await user.type(folderName, name)

    expect(
      await screen.findByText(
        'Folder name must not be "." or "..". Choose the parent folder from the list instead.',
      ),
    ).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Move' })).toBeDisabled()
  })

  it('sends nothing when cancelled', async () => {
    const { fetchMock, onClose } = renderDialog()
    const user = userEvent.setup()
    const folderName = screen.getByLabelText('Folder name')

    await user.clear(folderName)
    await user.type(folderName, 'lofi')
    await user.click(screen.getByRole('button', { name: 'Cancel' }))

    expect(onClose).toHaveBeenCalled()
    expect(sentRequests(fetchMock)).toEqual([])
  })
})
