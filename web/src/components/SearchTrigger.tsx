import { Search } from 'lucide-react'

/** The shortcut that opens search, as this platform writes it. */
const SEARCH_SHORTCUT_LABEL =
  typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform) ? '⌘K' : 'Ctrl K'

/**
 * The header's search control: a field-like button with the shortcut at the
 * small breakpoint and up, an icon button below it.
 */
export function SearchTrigger({ onOpen }: { onOpen: () => void }) {
  return (
    <button
      type="button"
      aria-label="Search"
      onClick={onOpen}
      className="flex size-9 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground sm:w-64 sm:justify-start sm:gap-2 sm:border sm:border-input sm:bg-background sm:px-3 sm:hover:bg-background sm:hover:text-foreground"
    >
      <Search aria-hidden="true" className="size-4 shrink-0" />
      <span aria-hidden="true" className="hidden flex-1 text-left text-sm sm:inline">
        Search…
      </span>
      <kbd
        aria-hidden="true"
        className="hidden rounded border border-border bg-muted px-1.5 font-sans text-[0.6875rem] text-muted-foreground sm:inline"
      >
        {SEARCH_SHORTCUT_LABEL}
      </kbd>
    </button>
  )
}
