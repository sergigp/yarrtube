import { NavLink, Outlet } from 'react-router-dom'
import { cn } from '@/lib/utils'

const SECTIONS = [
  { path: 'general', label: 'General' },
  { path: 'ai', label: 'AI' },
  { path: 'plex', label: 'Plex' },
] as const

/**
 * The Settings page: a nav of its sections, a column beside the open one on
 * wide viewports and a row of tabs above it on narrow ones.
 */
export function SettingsView() {
  return (
    <div className="flex w-full flex-col gap-6 md:flex-row md:gap-10">
      <nav
        aria-label="Settings sections"
        className="flex shrink-0 gap-1 overflow-x-auto border-b border-border md:w-40 md:flex-col md:border-b-0"
      >
        {SECTIONS.map(({ path, label }) => (
          <NavLink
            key={path}
            to={`/settings/${path}`}
            className={({ isActive }) =>
              cn(
                'whitespace-nowrap px-3 py-2 text-sm font-medium transition-colors',
                '-mb-px border-b-2 md:mb-0 md:rounded-md md:border-b-0',
                isActive
                  ? 'border-primary text-foreground md:bg-muted'
                  : 'border-transparent text-muted-foreground hover:text-foreground md:hover:bg-muted/50',
              )
            }
          >
            {label}
          </NavLink>
        ))}
      </nav>
      <div className="min-w-0 flex-1">
        <Outlet />
      </div>
    </div>
  )
}
