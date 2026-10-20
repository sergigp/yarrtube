import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { EditableTitle } from './EditableTitle'

describe('EditableTitle', () => {
  it('renames on Enter and on blur, cancels on Escape, closes unchanged without saving', async () => {
    const onRename = vi.fn(async () => {})
    render(<EditableTitle name="Chess" onRename={onRename} />)
    const user = userEvent.setup()
    const rename = async (keys: string) => {
      await user.click(screen.getByRole('button', { name: 'Rename' }))
      const field = screen.getByRole('textbox', { name: 'Name' })
      await user.clear(field)
      await user.type(field, keys)
    }

    await user.click(screen.getByRole('button', { name: 'Rename' }))
    const field = screen.getByRole('textbox', { name: 'Name' })
    const opened = {
      focused: field === document.activeElement,
      value: (field as HTMLInputElement).value,
    }
    await user.keyboard('{Enter}')
    await rename('Board games{Enter}')
    await rename('Openings')
    await user.tab()
    await rename('Endgames{Escape}')

    expect(opened).toEqual({ focused: true, value: 'Chess' })
    expect(onRename.mock.calls).toEqual([['Board games'], ['Openings']])
    expect(screen.queryByRole('textbox', { name: 'Name' })).not.toBeInTheDocument()
    expect(screen.getByRole('heading', { name: 'Chess' })).toBeInTheDocument()
  })

  it('keeps the field open with the message when the name is rejected', async () => {
    const onRename = vi.fn(async () => {
      throw new Error('a category named "Science" already exists')
    })
    render(<EditableTitle name="Chess" onRename={onRename} />)
    const user = userEvent.setup()

    await user.click(screen.getByRole('button', { name: 'Rename' }))
    await user.clear(screen.getByRole('textbox', { name: 'Name' }))
    await user.type(screen.getByRole('textbox', { name: 'Name' }), 'Science{Enter}')
    const message = (await screen.findByText(/already exists/)).textContent
    const field = screen.getByRole('textbox', { name: 'Name' })
    const rejected = {
      value: (field as HTMLInputElement).value,
      focused: field === document.activeElement,
    }
    await user.keyboard('{Escape}')

    expect(message).toBe('a category named "Science" already exists')
    expect(rejected).toEqual({ value: 'Science', focused: true })
    expect(screen.getByRole('heading', { name: 'Chess' })).toBeInTheDocument()
    expect(screen.queryByText(/already exists/)).not.toBeInTheDocument()
  })
})
