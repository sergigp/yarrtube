import { describe, expect, it } from 'vitest'
import { videoGridItemClassName, videoGridListClassName } from './videoGridLayout'

describe('videoGridLayout', () => {
  it('sizes every layout by the content area, not the viewport', () => {
    expect(
      (['grid', 'shelf', 'feed'] as const).map((layout) =>
        ['@xl:grid-cols-3', '@3xl:grid-cols-4', '@5xl:grid-cols-5', '@7xl:grid-cols-6'].every(
          (columns) => videoGridListClassName(layout).split(' ').includes(columns),
        ),
      ),
    ).toEqual([true, true, true])
  })

  it('shows a shelf card only once its column fits, so the shelf keeps to one row', () => {
    expect([0, 1, 2, 3, 4, 5, 6].map((index) => videoGridItemClassName('shelf', index))).toEqual([
      'max-sm:w-[92%] max-sm:shrink-0 max-sm:snap-start',
      'max-sm:w-[92%] max-sm:shrink-0 max-sm:snap-start',
      'max-sm:w-[92%] max-sm:shrink-0 max-sm:snap-start hidden max-sm:block @xl:block',
      'max-sm:w-[92%] max-sm:shrink-0 max-sm:snap-start hidden max-sm:block @3xl:block',
      'max-sm:w-[92%] max-sm:shrink-0 max-sm:snap-start hidden max-sm:block @5xl:block',
      'max-sm:w-[92%] max-sm:shrink-0 max-sm:snap-start hidden max-sm:block @7xl:block',
      'max-sm:w-[92%] max-sm:shrink-0 max-sm:snap-start hidden max-sm:block',
    ])
  })

  it('turns a shelf into a snapping horizontal scroller on phones', () => {
    expect(videoGridListClassName('shelf').split(' ')).toEqual(
      expect.arrayContaining([
        'max-sm:flex',
        'max-sm:overflow-x-auto',
        'max-sm:snap-x',
        'max-sm:snap-mandatory',
      ]),
    )
  })

  it('makes the feed one column on phones and keeps grid and feed cards always shown', () => {
    expect(videoGridListClassName('feed').split(' ')).toContain('max-sm:grid-cols-1!')
    expect(videoGridListClassName('grid').split(' ')).not.toContain('max-sm:grid-cols-1!')
    expect([videoGridItemClassName('grid', 9), videoGridItemClassName('feed', 9)]).toEqual(['', ''])
  })
})
