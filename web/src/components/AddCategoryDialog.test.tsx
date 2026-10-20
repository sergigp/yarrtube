import { describe, expect, it, vi } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { useLocation } from 'react-router-dom'
import { AddCategoryDialog } from './AddCategoryDialog'
import { mockApi, renderWithProviders, type Routes } from '@/test/helpers'

function RouteProbe() {
  const location = useLocation()
  return <output aria-label="Current route">{`${location.pathname}${location.search}`}</output>
}

function renderDialog(routes: Routes) {
  const fetchMock = mockApi(routes)
  const onOpenChange = vi.fn()
  renderWithProviders(
    <>
      <AddCategoryDialog open onOpenChange={onOpenChange} />
      <RouteProbe />
    </>,
  )
  return { fetchMock, onOpenChange }
}

describe('AddCategoryDialog', () => {
  it('creates a category and opens it in the management view', async () => {
    const create = vi.fn(() => ({ id: 'chess', name: 'Chess', channels: [], playlists: [] }))
    const { fetchMock, onOpenChange } = renderDialog({ 'POST /api/categories': create })
    const user = userEvent.setup()

    await user.type(screen.getByLabelText('Name'), 'Chess')
    await user.click(screen.getByRole('button', { name: 'Create' }))

    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false))
    const [, init] = fetchMock.mock.calls[0] as [string, RequestInit]
    expect(JSON.parse(init.body as string)).toEqual({ name: 'Chess' })
    expect(screen.getByRole('status', { name: 'Current route', hidden: true })).toHaveTextContent(
      '/categories?selected=chess',
    )
  })
})
