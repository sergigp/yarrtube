import { useState } from 'react'
import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
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
    render(<Picker initial="#fff1bf" onChange={onChange} />)
    const user = userEvent.setup()

    await user.click(screen.getByRole('radio', { name: 'Sky' }))

    expect(onChange).toHaveBeenCalledWith('#d3ecf8')
    expect(screen.getByRole('radio', { name: 'Sky' })).toBeChecked()
    expect(screen.getByRole('radio', { name: 'Butter' })).not.toBeChecked()
  })
})
