import { describe, expect, it, vi } from 'vitest'
import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { FavoriteButton } from './FavoriteButton'
import { mockApi, renderWithProviders } from '@/test/helpers'

describe('FavoriteButton', () => {
  it('favorites a video', async () => {
    const favorite = vi.fn(() => null)
    mockApi({ 'PUT /api/videos/v1/favorite': favorite })
    renderWithProviders(<FavoriteButton videoId="v1" title="My Video" favorite={false} />)

    await userEvent.setup().click(screen.getByRole('button', { name: 'Add to favorites' }))

    expect(favorite).toHaveBeenCalledOnce()
    expect(await screen.findByRole('button', { name: 'Remove from favorites' })).toHaveAttribute(
      'aria-pressed',
      'true',
    )
  })
  it('unfavorites a favorite without asking', async () => {
    const unfavorite = vi.fn(() => null)
    mockApi({ 'DELETE /api/videos/v1/favorite': unfavorite })
    renderWithProviders(<FavoriteButton videoId="v1" title="My Video" favorite />)

    await userEvent.setup().click(screen.getByRole('button', { name: 'Remove from favorites' }))

    expect(unfavorite).toHaveBeenCalledOnce()
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
    expect(await screen.findByRole('button', { name: 'Add to favorites' })).toHaveAttribute(
      'aria-pressed',
      'false',
    )
  })
})
