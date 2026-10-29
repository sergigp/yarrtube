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

/** The chosen categories' chips, in name order. */
function chosen(): string[] {
  return screen.getAllByRole('button', { pressed: true }).map((chip) => chip.textContent ?? '')
}

describe('CategoryPicker', () => {
  it('shows every category, toggles them and colors the chosen ones', async () => {
    mockApi({
      'GET /api/categories': aCategoryList([
        aCategory({ id: 'streams', name: 'Streams', color: '#fff1bf' }),
        aCategory({ id: 'chess', name: 'Chess', color: '#d3ecf8' }),
        aCategory({ id: 'science', name: 'Science', color: '#e9def8' }),
      ]),
    })
    renderWithProviders(<Harness initial={['chess']} />)
    const user = userEvent.setup()
    const group = await screen.findByRole('group', { name: 'Categories' })
    await within(group).findByRole('button', { name: 'Chess' })
    const shown = within(within(group).getByRole('list'))
      .getAllByRole('button')
      .map((chip) => chip.textContent)
    const initial = chosen()

    await user.click(screen.getByRole('button', { name: 'Streams' }))
    const afterPick = chosen()
    await user.click(screen.getByRole('button', { name: 'Chess' }))

    expect(shown).toEqual(['Chess', 'Science', 'Streams'])
    expect(initial).toEqual(['Chess'])
    expect(afterPick).toEqual(['Chess', 'Streams'])
    expect(chosen()).toEqual(['Streams'])
    expect(screen.getByRole('button', { name: 'Streams' })).toHaveStyle({
      background: '#fff1bf',
    })
    expect(screen.getByRole('button', { name: 'Chess' }).style.background).toBe('')
  })

  it('creates a category and chooses it, from the button or Enter', async () => {
    let categories = [aCategory({ id: 'chess', name: 'Chess' })]
    const toCreate = ['Speedcubing', 'Crafts']
    const fetchMock = mockApi({
      'GET /api/categories': () => aCategoryList(categories),
      'POST /api/categories': () => {
        const name = toCreate.shift() as string
        const id = name.toLowerCase()
        categories = [...categories, aCategory({ id, name })]
        return aCreatedCategory({ id, name })
      },
    })
    renderWithProviders(<Harness initial={[]} />)
    const user = userEvent.setup()
    const field = screen.getByRole('textbox', { name: 'New category' })
    await screen.findByRole('button', { name: 'Chess' })

    await user.type(field, 'Speedcubing')
    await user.click(screen.getByRole('button', { name: 'Add' }))
    await screen.findByRole('button', { name: 'Speedcubing', pressed: true })
    await user.type(field, 'Crafts{Enter}')
    await screen.findByRole('button', { name: 'Crafts', pressed: true })

    expect(chosen()).toEqual(['Crafts', 'Speedcubing'])
    expect(field).toHaveValue('')
    expect(
      fetchMock.mock.calls
        .filter(([, init]) => (init as RequestInit | undefined)?.method === 'POST')
        .map(([, init]) => JSON.parse((init as RequestInit).body as string)),
    ).toEqual([{ name: 'Speedcubing' }, { name: 'Crafts' }])
  })

  it('chooses the existing category when its name is typed', async () => {
    const fetchMock = mockApi({
      'GET /api/categories': aCategoryList([aCategory({ id: 'chess', name: 'Chess' })]),
    })
    renderWithProviders(<Harness initial={[]} />)
    const user = userEvent.setup()
    await screen.findByRole('button', { name: 'Chess' })

    await user.type(screen.getByRole('textbox', { name: 'New category' }), 'chess{Enter}')

    expect(chosen()).toEqual(['Chess'])
    expect(
      fetchMock.mock.calls.filter(
        ([, init]) => (init as RequestInit | undefined)?.method === 'POST',
      ),
    ).toEqual([])
  })
})
