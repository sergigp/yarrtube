import { screen, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { aCategoryVideo, mockApi, renderWithProviders } from '@/test/helpers'
import { CategoryPlayer } from './CategoryPlayer'

describe('CategoryPlayer', () => {
  it('shows the rail in the category player', async () => {
    mockApi({ 'GET /api/channels': [] })
    const video = aCategoryVideo({
      chapters: [
        { start_seconds: 0, end_seconds: 75, title: 'Intro' },
        { start_seconds: 75, end_seconds: 1037, title: 'Game 1' },
      ],
    })

    renderWithProviders(<CategoryPlayer videos={[video]} error={null} />)

    const rail = await screen.findByRole('navigation', { name: 'Chapters' })
    expect(
      within(within(rail).getByRole('list'))
        .getAllByRole('button')
        .map((card) => card.textContent),
    ).toEqual(['0:00 Intro', '1:15 Game 1'])
  })
})
