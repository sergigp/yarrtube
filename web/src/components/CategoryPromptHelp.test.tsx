import { describe, expect, it } from 'vitest'
import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { CategoryPromptHelp } from './CategoryPromptHelp'

describe('CategoryPromptHelp', () => {
  it('opens a guide with a good example, bad examples with their reasons, and tips', async () => {
    render(<CategoryPromptHelp />)
    const user = userEvent.setup()

    await user.click(screen.getByRole('button', { name: 'About AI prompt' }))

    const guide = within(await screen.findByRole('region', { name: 'AI prompt guide' }))
    const good = within(guide.getByRole('group', { name: 'A good prompt' }))
    expect(good.getByText(/^Belongs:/)).toBeInTheDocument()
    expect(good.getByText(/^Does not belong:/)).toBeInTheDocument()
    const bad = within(guide.getByRole('group', { name: 'Bad prompts' }))
    expect(bad.getByText('Chess')).toBeInTheDocument()
    expect(bad.getByText(/Too vague/)).toBeInTheDocument()
    expect(bad.getByText('Videos I would enjoy')).toBeInTheDocument()
    expect(bad.getByText(/Only you know/)).toBeInTheDocument()
    expect(guide.getByText(/what is close but doesn't belong/)).toBeInTheDocument()
    expect(guide.getByText(/words a title, chapter or summary would contain/)).toBeInTheDocument()
    expect(guide.getByText(/whether one part of a video is enough/)).toBeInTheDocument()
  })
})
