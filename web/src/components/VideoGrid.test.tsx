import { describe, expect, it } from 'vitest'
import { screen } from '@testing-library/react'
import { VideoGrid } from './VideoGrid'
import { aHomeVideo, mockApi, renderWithProviders } from '@/test/helpers'

/** The card (list item) holding the video titled `title`. */
function card(title: string): HTMLElement {
  return screen.getByRole('link', { name: title }).closest('li') as HTMLElement
}

describe('VideoGrid', () => {
  it('draws cards frameless: no border, no background of their own', () => {
    mockApi({})
    renderWithProviders(<VideoGrid videos={[aHomeVideo({ title: 'Fresh Video' })]} />)

    expect(card('Fresh Video').style.background).toBe('')
    expect(card('Fresh Video').className.split(' ')).not.toContain('border')
    expect(
      card('Fresh Video')
        .className.split(' ')
        .filter((name) => /^(bg-|border)/.test(name)),
    ).toEqual([])
  })

  it('shows a hover surface behind the card and zooms the thumbnail within its rounded bounds', () => {
    mockApi({})
    renderWithProviders(<VideoGrid videos={[aHomeVideo({ title: 'Fresh Video' })]} />)

    expect(card('Fresh Video')).toHaveClass('hover:bg-muted/60')
    const thumbnailLink = card('Fresh Video').querySelector('[data-slot="card-thumbnail"]')
    expect(thumbnailLink).toHaveClass('overflow-hidden', 'rounded-lg')
    expect(thumbnailLink?.firstElementChild).toHaveClass('group-hover/card:scale-[1.03]')
  })
})
