import { describe, expect, it } from 'vitest'
import { orderCategories } from './categories'
import { aCategory } from '@/test/helpers'

describe('categories', () => {
  it('orders unread first, most first, ties and caught-up by name ignoring case', () => {
    const categories = [
      aCategory({ name: 'Alpha', unwatched_count: 0 }),
      aCategory({ name: 'sports', unwatched_count: 4 }),
      aCategory({ name: 'Charlie', unwatched_count: 12 }),
      aCategory({ name: 'delta', unwatched_count: 0 }),
      aCategory({ name: 'Cooking', unwatched_count: 4 }),
      aCategory({ name: 'Bravo', unwatched_count: 0 }),
    ]

    const ordered = orderCategories(categories)

    expect(ordered.map((category) => category.name)).toEqual([
      'Charlie',
      'Cooking',
      'sports',
      'Alpha',
      'Bravo',
      'delta',
    ])
    expect(categories.map((category) => category.name)[0]).toBe('Alpha')
  })
})
