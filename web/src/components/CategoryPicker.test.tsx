import { useState } from 'react'
import { describe, expect, it } from 'vitest'
import { screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { CategoryPicker } from './CategoryPicker'
import {
  aCategory,
  aCategoryList,
  aCreatedCategory,
  mockApi,
  renderWithProviders,
} from '@/test/helpers'

function Harness({ initial }: { initial: string[] }) {
  const [value, setValue] = useState(initial)
  return <CategoryPicker value={value} onChange={setValue} />
}

function chips(): string[] {
  return within(screen.getByRole('list', { name: 'Chosen categories' }))
    .queryAllByRole('listitem')
    .map((chip) => chip.textContent ?? '')
}

describe('CategoryPicker', () => {
  it('picks, removes and creates categories', async () => {
    let categories = [
      aCategory({ id: 'chess', name: 'Chess' }),
      aCategory({ id: 'science', name: 'Science' }),
      aCategory({ id: 'streams', name: 'Streams' }),
    ]
    const fetchMock = mockApi({
      'GET /api/categories': () => aCategoryList(categories),
      'POST /api/categories': () => {
        categories = [...categories, aCategory({ id: 'speedcubing', name: 'Speedcubing' })]
        return aCreatedCategory({ id: 'speedcubing', name: 'Speedcubing' })
      },
    })
    renderWithProviders(<Harness initial={['chess']} />)
    const user = userEvent.setup()
    const field = await screen.findByRole('combobox', { name: 'Categories' })
    await screen.findByText('Chess')
    const initialChips = chips()

    await user.type(field, 'st')
    const options = screen.getAllByRole('option').map((option) => option.textContent)
    await user.click(screen.getByRole('option', { name: 'Streams' }))
    const afterPick = chips()
    await user.click(screen.getByRole('button', { name: 'Remove Chess' }))
    const afterRemove = chips()
    await user.type(field, 'Speedcubing')
    await user.click(screen.getByRole('option', { name: 'Create "Speedcubing"' }))

    expect(await screen.findByText('Speedcubing')).toBeInTheDocument()
    expect(initialChips).toEqual(['Chess'])
    expect(options).toEqual(['Streams', 'Create "st"'])
    expect(afterPick).toEqual(['Chess', 'Streams'])
    expect(afterRemove).toEqual(['Streams'])
    expect(chips()).toEqual(['Streams', 'Speedcubing'])
    expect(field).toHaveValue('')
    const [, init] = fetchMock.mock.calls.find(
      ([, init]) => (init as RequestInit | undefined)?.method === 'POST',
    ) as [string, RequestInit]
    expect(JSON.parse(init.body as string)).toEqual({ name: 'Speedcubing' })
  })

  it('picks and creates categories from the keyboard', async () => {
    let categories = [
      aCategory({ id: 'chess', name: 'Chess' }),
      aCategory({ id: 'cooking', name: 'Cooking' }),
    ]
    mockApi({
      'GET /api/categories': () => aCategoryList(categories),
      'POST /api/categories': () => {
        categories = [...categories, aCategory({ id: 'crafts', name: 'Crafts' })]
        return aCreatedCategory({ id: 'crafts', name: 'Crafts' })
      },
    })
    renderWithProviders(<Harness initial={[]} />)
    const user = userEvent.setup()
    const field = await screen.findByRole('combobox', { name: 'Categories' })

    await user.type(field, 'c')
    const highlighted = screen.getByRole('option', { selected: true }).textContent
    await user.keyboard('{ArrowDown}{Enter}')
    const afterArrowPick = chips()
    await user.type(field, 'che{Enter}')
    await user.type(field, 'Crafts{Enter}')

    expect(await screen.findByText('Crafts')).toBeInTheDocument()
    expect(highlighted).toBe('Chess')
    expect(afterArrowPick).toEqual(['Cooking'])
    expect(chips()).toEqual(['Cooking', 'Chess', 'Crafts'])
  })
})
