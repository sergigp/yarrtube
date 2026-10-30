/**
 * How a video grid lays its cards out: `grid` wraps into rows; `shelf` keeps
 * to one row, as many cards as fit, and swipes on phones; `feed` is a grid
 * that becomes one column on phones.
 */
export type VideoGridLayout = 'grid' | 'shelf' | 'feed'

/**
 * Columns by the content area's width (the `@container` around the routes),
 * so the sidebar never skews them: each entry's class applies from its
 * container breakpoint up, and is the one that shows the card at its index
 * on a shelf. Both lists are written out whole so Tailwind finds the classes.
 */
const COLUMNS = [
  { count: 2, columns: 'grid-cols-2', shows: '' },
  { count: 3, columns: '@xl:grid-cols-3', shows: '@xl:block' },
  { count: 4, columns: '@3xl:grid-cols-4', shows: '@3xl:block' },
  { count: 5, columns: '@5xl:grid-cols-5', shows: '@5xl:block' },
  { count: 6, columns: '@7xl:grid-cols-6', shows: '@7xl:block' },
]

const GRID = ['grid gap-x-4 gap-y-6', ...COLUMNS.map((step) => step.columns)].join(' ')

/** Below `sm`, a shelf scrolls sideways from edge to edge, snapping card by card. */
const PHONE_SHELF =
  'max-sm:-mx-4 max-sm:flex max-sm:snap-x max-sm:snap-mandatory max-sm:scroll-px-4 max-sm:gap-3 max-sm:overflow-x-auto max-sm:px-4'

/** Nearly the full width, leaving a sliver of the next card to show the shelf swipes. */
const PHONE_SHELF_CARD = 'max-sm:w-[92%] max-sm:shrink-0 max-sm:snap-start'

export function videoGridListClassName(layout: VideoGridLayout): string {
  switch (layout) {
    case 'grid':
      return GRID
    case 'shelf':
      return `${GRID} ${PHONE_SHELF}`
    case 'feed':
      return `${GRID} max-sm:grid-cols-1!`
  }
}

/**
 * A card's classes at `index`: on a shelf, hidden at `sm` and up until its
 * column fits, so the shelf never starts a second row.
 */
export function videoGridItemClassName(layout: VideoGridLayout, index: number): string {
  if (layout !== 'shelf') {
    return ''
  }
  const fitsFrom = COLUMNS.find((step) => step.count > index)
  if (fitsFrom && !fitsFrom.shows) {
    return PHONE_SHELF_CARD
  }
  return [PHONE_SHELF_CARD, 'hidden max-sm:block', fitsFrom?.shows].filter(Boolean).join(' ')
}
