import { ListFilter, X } from 'lucide-react'

interface SidebarFilterProps {
  value: string
  onChange: (value: string) => void
  /** Whether the categories show, so the field names them too. */
  withCategories: boolean
}

/**
 * The field at the top of the sidebar narrowing its sections by name.
 * Escape clears it before it would close the mobile drawer.
 */
export function SidebarFilter({ value, onChange, withCategories }: SidebarFilterProps) {
  const label = withCategories
    ? 'Filter categories, channels and playlists'
    : 'Filter channels and playlists'
  return (
    <label className="mx-1 mb-3 flex h-9 shrink-0 items-center gap-2 rounded-lg border border-input px-2.5 text-muted-foreground transition-colors focus-within:border-ring focus-within:ring-3 focus-within:ring-ring/50">
      <ListFilter aria-hidden="true" className="size-3.5 shrink-0" />
      <input
        type="search"
        value={value}
        onChange={(event) => onChange(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === 'Escape' && value !== '') {
            event.preventDefault()
            onChange('')
          }
        }}
        aria-label={label}
        placeholder={withCategories ? 'Filter categories, channels…' : 'Filter channels, playlists…'}
        className="min-w-0 flex-1 bg-transparent text-sm text-foreground outline-none placeholder:text-muted-foreground [&::-webkit-search-cancel-button]:hidden"
      />
      {value !== '' && (
        <button
          type="button"
          aria-label="Clear filter"
          onClick={() => onChange('')}
          className="-mr-1 flex size-6 shrink-0 items-center justify-center rounded-md transition-colors hover:bg-secondary hover:text-foreground"
        >
          <X aria-hidden="true" className="size-3.5" />
        </button>
      )}
    </label>
  )
}
