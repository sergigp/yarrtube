/** What a chip shows of a category. */
export interface CategoryChip {
  name: string
  color: string
}

/**
 * A video's categories on one line: the first as a dot in its colour and its
 * name, then "+N" for the rest, so every card in a row keeps the same height.
 * Nothing for a video in no known category.
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
  const [first, ...rest] = known
  return (
    <ul
      aria-label="Categories"
      className="flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground"
    >
      <li className="flex min-w-0 items-center gap-1.5">
        <span
          aria-hidden="true"
          className="size-1.5 shrink-0 rounded-full"
          style={{ background: first.color }}
        />
        <span className="truncate">{first.name}</span>
      </li>
      {rest.length > 0 && (
        <li
          className="shrink-0 rounded-full bg-muted px-1.5 leading-4"
          title={rest.map((chip) => chip.name).join(', ')}
        >
          +{rest.length}
        </li>
      )}
    </ul>
  )
}
