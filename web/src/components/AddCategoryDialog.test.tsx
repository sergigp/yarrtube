import { describe, expect, it, vi } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { AddCategoryDialog } from './AddCategoryDialog'
import { aCreatedCategory, mockApi, renderWithProviders, type Routes } from '@/test/helpers'
import { RouteProbe } from '@/test/RouteProbe'

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
  it('creates a category and opens its view', async () => {
    const create = vi.fn(() => (aCreatedCategory({ id: 'chess', name: 'Chess' })))
    const { fetchMock, onOpenChange } = renderDialog({ 'POST /api/categories': create })
    const user = userEvent.setup()

    await user.type(screen.getByLabelText('Name'), 'Chess')
    await user.click(screen.getByRole('button', { name: 'Create' }))

    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false))
    const [, init] = fetchMock.mock.calls[0] as [string, RequestInit]
    expect(JSON.parse(init.body as string)).toEqual({ name: 'Chess' })
    expect(screen.getByRole('status', { name: 'Current route', hidden: true })).toHaveTextContent(
      '/categories/chess',
    )
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
