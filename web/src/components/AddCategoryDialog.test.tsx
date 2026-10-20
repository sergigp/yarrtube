import { describe, expect, it, vi } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Route, Routes as RouterRoutes } from 'react-router-dom'
import { AddCategoryDialog } from './AddCategoryDialog'
import { CategoryDetail } from './CategoryDetail'
import {
  aCategory,
  aCategoryList,
  aCreatedCategory,
  mockApi,
  renderWithProviders,
  type Routes as ApiRoutes,
} from '@/test/helpers'
import { RouteProbe } from '@/test/RouteProbe'

function renderDialog(routes: ApiRoutes) {
  const fetchMock = mockApi(routes)
  const onOpenChange = vi.fn()
  renderWithProviders(
    <>
      <AddCategoryDialog open onOpenChange={onOpenChange} />
      <RouterRoutes>
        <Route path="/categories/:id" element={<CategoryDetail />} />
      </RouterRoutes>
      <RouteProbe />
    </>,
  )
  return { fetchMock, onOpenChange }
}

describe('AddCategoryDialog', () => {
  it('creates a category and opens it with the manage sources dialog', async () => {
    const create = vi.fn(() => (aCreatedCategory({ id: 'chess', name: 'Chess' })))
    const { fetchMock, onOpenChange } = renderDialog({
      'POST /api/categories': create,
      'GET /api/categories': aCategoryList([aCategory({ id: 'chess', name: 'Chess' })]),
      'GET /api/categories/chess/videos': [],
      'GET /api/channels': [],
      'GET /api/playlists': [],
    })
    const user = userEvent.setup()

    await user.type(screen.getByLabelText('Name'), 'Chess')
    await user.click(screen.getByRole('button', { name: 'Create' }))

    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false))
    const [, init] = fetchMock.mock.calls[0] as [string, RequestInit]
    expect(JSON.parse(init.body as string)).toEqual({ name: 'Chess' })
    expect(screen.getByRole('status', { name: 'Current route', hidden: true })).toHaveTextContent(
      '/categories/chess',
    )
    expect(await screen.findByRole('dialog', { name: 'Manage sources in Chess' })).toBeVisible()
    expect(screen.getByRole('searchbox', { name: 'Search channels and playlists' })).toHaveFocus()
  })

  it('disables create for a blank name and shows a taken-name error', async () => {
    const { onOpenChange } = renderDialog({
      'POST /api/categories': { status: 409, error: 'a category named "Chess" already exists' },
    })
    const user = userEvent.setup()
    const create = screen.getByRole('button', { name: 'Create' })
    const disabledWhenEmpty = create.hasAttribute('disabled')
    await user.type(screen.getByLabelText('Name'), '   ')
    const disabledWhenBlank = create.hasAttribute('disabled')

    await user.type(screen.getByLabelText('Name'), 'Chess')
    await user.click(create)

    expect([disabledWhenEmpty, disabledWhenBlank]).toEqual([true, true])
    expect(
      await screen.findByText('a category named "Chess" already exists'),
    ).toBeInTheDocument()
    expect(onOpenChange).not.toHaveBeenCalled()
  })
})
