import { useState } from 'react'
import { describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { CategoryColorPicker } from './CategoryColorPicker'

/** The picker as its callers hold it: the chosen color in state. */
function Picker({ initial, onChange }: { initial: string; onChange: (hex: string) => void }) {
  const [value, setValue] = useState(initial)
  return (
    <CategoryColorPicker
      value={value}
      onChange={(hex) => {
        setValue(hex)
        onChange(hex)
      }}
    />
  )
}

describe('CategoryColorPicker', () => {
  it('checks a chosen swatch and reports its color', async () => {
    const onChange = vi.fn()
    render(<Picker initial="#9f8401" onChange={onChange} />)
    const user = userEvent.setup()

    await user.click(screen.getByRole('radio', { name: 'Sky' }))

    expect(onChange).toHaveBeenCalledWith('#0193bf')
    expect(screen.getByRole('radio', { name: 'Sky' })).toBeChecked()
    expect(screen.getByRole('radio', { name: 'Mustard' })).not.toBeChecked()
  })

  it('shows a color outside the palette as Custom, whose hue slider picks solid colors', () => {
    const onChange = vi.fn()
    render(<Picker initial="#f0e4d0" onChange={onChange} />)

    expect(screen.getByRole('radio', { name: 'Custom' })).toBeChecked()
    expect(screen.getByRole('slider', { name: 'Hue' })).toHaveValue('81')

    fireEvent.change(screen.getByRole('slider', { name: 'Hue' }), { target: { value: '200' } })

    expect(onChange).toHaveBeenCalledWith('#03999f')
    expect(screen.getByRole('radio', { name: 'Custom' })).toBeChecked()
    expect(screen.getByRole('slider', { name: 'Hue' })).toHaveValue('200')
  })
})
