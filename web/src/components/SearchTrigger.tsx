import { Search } from 'lucide-react'

/** The shortcut that opens search, as this platform writes it. */
const SEARCH_SHORTCUT_LABEL =
  typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform) ? '⌘K' : 'Ctrl K'

/**
 * The header's search control: a wide pill with the shortcut, centred in the
 * header, at the small breakpoint and up; an icon button below it.
 */
export function SearchTrigger({ onOpen }: { onOpen: () => void }) {
  return (
    <button
      type="button"
      aria-label="Search"
      onClick={onOpen}
      className="flex size-9 items-center justify-center rounded-full text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground sm:h-10 sm:w-full sm:max-w-140 sm:justify-start sm:gap-2.5 sm:border sm:border-input sm:bg-card sm:pr-3 sm:pl-4 sm:hover:border-ring/60 sm:hover:bg-card"
    >
      <Search aria-hidden="true" className="size-4.5 shrink-0" />
      <span aria-hidden="true" className="hidden min-w-0 flex-1 truncate text-left text-sm sm:inline">
        Search videos, channels, playlists
      </span>
      <kbd
        aria-hidden="true"
        className="hidden shrink-0 rounded-md border border-input px-1.5 font-sans text-xs text-muted-foreground sm:inline"
      >
        {SEARCH_SHORTCUT_LABEL}
      </kbd>
    </button>
  )
}
