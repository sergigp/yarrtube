import { inkShade, softTint } from '@/lib/categoryColors'

/** What a chip shows of a category. */
export interface CategoryChip {
  name: string
  color: string
}

/** Chips shown before the rest collapse into "+N". */
const SHOWN_CHIPS = 2

/**
 * A video's categories, in its order, as small chips in each category's hue:
 * at most 2, then "+N" for the rest. Nothing for a video in no known category.
 */
export function CategoryChips({
  categoryIds,
  chips,
}: {
  categoryIds: string[]
  chips: Map<string, CategoryChip>
}) {
  const known = categoryIds.flatMap((id) => chips.get(id) ?? [])
  if (known.length === 0) {
    return null
  }
  const hidden = known.length - SHOWN_CHIPS
  return (
    <ul aria-label="Categories" className="mt-1 flex min-w-0 flex-wrap gap-1">
      {known.slice(0, SHOWN_CHIPS).map((chip) => (
        <li
          key={chip.name}
          className="max-w-full truncate rounded px-1.5 py-px text-[0.6875rem] leading-4 font-medium"
          style={{ background: softTint(chip.color), color: inkShade(chip.color) }}
        >
          {chip.name}
        </li>
      ))}
      {hidden > 0 && (
        <li className="rounded bg-muted px-1.5 py-px text-[0.6875rem] leading-4 font-medium text-muted-foreground">
          +{hidden}
        </li>
      )}
    </ul>
  )
}
