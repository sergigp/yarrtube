import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MobileNavToggle } from './MobileNavToggle'

describe('MobileNavToggle', () => {
  it('offers to open the menu while closed', () => {
    render(<MobileNavToggle open={false} onToggle={() => {}} controls="nav" />)

    const toggle = screen.getByRole('button', { name: 'Open menu' })
    expect(toggle).toHaveAttribute('aria-expanded', 'false')
    expect(toggle).toHaveAttribute('aria-controls', 'nav')
  })

  it('offers to close the menu while open', () => {
    render(<MobileNavToggle open onToggle={() => {}} controls="nav" />)

    expect(screen.getByRole('button', { name: 'Close menu' })).toHaveAttribute(
      'aria-expanded',
      'true',
    )
  })

  it('toggles when clicked', async () => {
    const user = userEvent.setup()
    const onToggle = vi.fn()
    render(<MobileNavToggle open={false} onToggle={onToggle} controls="nav" />)

    await user.click(screen.getByRole('button', { name: 'Open menu' }))

    expect(onToggle).toHaveBeenCalledOnce()
  })
})
