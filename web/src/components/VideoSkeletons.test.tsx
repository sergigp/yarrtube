import { describe, expect, it } from 'vitest'
import { render, screen } from '@testing-library/react'
import { VideoGridSkeleton, VideoRowsSkeleton } from './VideoSkeletons'

describe('VideoSkeletons', () => {
  it('announces a grid of placeholder cards as loading, marked busy', () => {
    render(<VideoGridSkeleton count={4} />)

    const status = screen.getByRole('status')
    expect(status).toHaveTextContent('Loading…')
    expect(status.closest('[aria-busy="true"]')).not.toBeNull()
  })

  it('announces placeholder list rows as loading, marked busy', () => {
    render(<VideoRowsSkeleton count={3} />)

    const status = screen.getByRole('status')
    expect(status).toHaveTextContent('Loading…')
    expect(status.closest('[aria-busy="true"]')).not.toBeNull()
  })
})
