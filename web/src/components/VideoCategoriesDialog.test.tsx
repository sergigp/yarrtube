import { describe, expect, it } from 'vitest'
import { screen, within } from '@testing-library/react'
import type { VideoCategoryChoice } from '@/api/types'
import { VideoCategoriesDialog } from './VideoCategoriesDialog'
import {
  aPreferences,
  aVideoCategoryChoice,
  mockApi,
  renderWithProviders,
  type Routes,
} from '@/test/helpers'

function renderDialog(routes: Routes) {
  const fetchMock = mockApi({ 'GET /api/preferences': aPreferences(), ...routes })
  renderWithProviders(
    <VideoCategoriesDialog
      recordId="rec1"
      title="Najdorf in 10 minutes"
      sourceName="@GothamChess"
      open
      onOpenChange={() => {}}
    />,
  )
  return { fetchMock }
}

/** Each chip as its accessible name, whether it's filled and whether it's locked. */
async function chips(): Promise<[string, boolean, boolean][]> {
  return within(await screen.findByRole('toolbar', { name: 'Categories' }))
    .getAllByRole('button')
    .map((chip) => [
      chip.getAttribute('aria-label') ?? chip.textContent ?? '',
      chip.getAttribute('aria-pressed') === 'true',
      chip.hasAttribute('disabled'),
    ])
}

const chess = aVideoCategoryChoice({ id: 'chess', name: 'Chess', belongs: true, from: 'source' })
const funny = aVideoCategoryChoice({ id: 'funny', name: 'Funny' })
const sicilian = aVideoCategoryChoice({
  id: 'sicilian',
  name: 'Sicilian',
  belongs: true,
  from: 'ai',
  verdict: 'yes',
})

function categoriesOf(...categories: VideoCategoryChoice[]) {
  return { categories }
}

describe('VideoCategoriesDialog', () => {
  it('shows locked source chips, AI marks and filled chips', async () => {
    renderDialog({ 'GET /api/videos/rec1/categories': categoriesOf(chess, funny, sicilian) })

    expect(
      screen.getByRole('dialog', { name: 'Najdorf in 10 minutes' }),
    ).toBeInTheDocument()
    expect(await chips()).toEqual([
      ['Chess, from @GothamChess', true, true],
      ['Funny', false, false],
      ['Sicilian, decided by AI', true, false],
    ])
    expect(screen.getByText('From @GothamChess')).toBeInTheDocument()
    expect(screen.getByText('AI')).toBeInTheDocument()
  })
})
