import { describe, expect, it } from 'vitest'
import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { SaveToField } from './SaveToField'
import { useSaveLocation, type LocationValue } from '@/hooks/useSaveLocation'
import { mockApi, renderWithProviders, type Routes } from '@/test/helpers'

const baseRoutes: Routes = {
  'GET /api/playlists': [],
  'GET /api/channels': [],
  'GET /api/directories': { root: '/videos', path: '', entries: [] },
  'GET /api/directories?path=playlists': { root: '/videos', path: 'playlists', entries: [] },
}

/** Renders the field on a real hook and captures the composed value. */
function renderField(routes: Routes) {
  mockApi({ ...baseRoutes, ...routes })
  const captured: { value?: LocationValue } = {}
  function Harness() {
    const location = useSaveLocation('playlist', 'My Mix')
    captured.value = location.value
    return <SaveToField location={location} />
  }
  renderWithProviders(<Harness />)
  return captured
}

describe('SaveToField', () => {
  it('stages a new folder that does not exist yet', async () => {
    const captured = renderField({})
    const user = userEvent.setup()

    await user.click(screen.getByRole('button', { name: 'Choose another folder…' }))
    await user.click(await screen.findByRole('button', { name: 'New folder' }))
    await user.type(screen.getByLabelText('New folder name'), 'fresh')
    await user.click(screen.getByRole('button', { name: 'Use folder' }))

    expect(
      await screen.findByText('Does not exist yet — created on the first download.'),
    ).toBeInTheDocument()
    expect(captured.value).toEqual({
      path: 'playlists/fresh/my-mix',
      destination: '/videos/playlists/fresh/my-mix',
      valid: true,
      occupiedBy: undefined,
    })

    await user.click(screen.getByRole('button', { name: 'Use this folder' }))
    expect(
      screen.getByRole('radio', { name: 'playlists/fresh/', checked: true }),
    ).toBeInTheDocument()
  })

  it('descends into an existing directory named in the create-folder step', async () => {
    renderField({
      'GET /api/directories?path=playlists': {
        root: '/videos',
        path: 'playlists',
        entries: [{ name: 'kids' }],
      },
      'GET /api/directories?path=playlists%2Fkids': {
        root: '/videos',
        path: 'playlists/kids',
        entries: [],
      },
    })
    const user = userEvent.setup()

    await user.click(screen.getByRole('button', { name: 'Choose another folder…' }))
    await user.click(await screen.findByRole('button', { name: 'New folder' }))
    await user.type(screen.getByLabelText('New folder name'), 'kids')
    await user.click(screen.getByRole('button', { name: 'Use folder' }))

    // An existing name is a way of reaching that directory, not a new one.
    expect(await screen.findByText('No subfolders here.')).toBeInTheDocument()
    expect(
      screen.queryByText('Does not exist yet — created on the first download.'),
    ).not.toBeInTheDocument()
  })

  it('rejects a new folder name containing a slash', async () => {
    renderField({})
    const user = userEvent.setup()

    await user.click(screen.getByRole('button', { name: 'Choose another folder…' }))
    await user.click(await screen.findByRole('button', { name: 'New folder' }))
    await user.type(screen.getByLabelText('New folder name'), 'a/b')
    await user.click(screen.getByRole('button', { name: 'Use folder' }))

    expect(await screen.findByText('Folder name must not contain "/".')).toBeInTheDocument()
  })
})
