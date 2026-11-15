import { NavLink, Outlet } from 'react-router-dom'
import { cn } from '@/lib/utils'
import { usePlexStatus, usePreferences } from '@/api/queries'
import { aiStatus, plexStatus } from '@/lib/settingsStatus'

const SECTIONS = [
  { path: 'general', label: 'General' },
  { path: 'ai', label: 'AI' },
  { path: 'plex', label: 'Plex' },
] as const

type SectionPath = (typeof SECTIONS)[number]['path']

/**
 * The Settings page: a nav of its sections, each with its status, as a column
 * beside the open one on wide viewports and a row of tabs above it on narrow
 * ones (without the statuses).
 */
export function SettingsView() {
  const { data: preferences } = usePreferences()
  const { data: plex } = usePlexStatus()
  const statuses: Record<SectionPath, string | null> = {
    general: 'Home and categories',
    ai: aiStatus(preferences),
    plex: plexStatus(plex),
  }
  return (
    <div className="flex w-full flex-col gap-6 md:flex-row md:gap-10">
      <nav
        aria-label="Settings sections"
        className="flex shrink-0 gap-1 overflow-x-auto border-b border-border md:w-52 md:flex-col md:border-b-0"
      >
        {SECTIONS.map(({ path, label }) => {
          const status = statuses[path]
          return (
            <NavLink
              key={path}
              to={`/settings/${path}`}
              aria-label={label}
              aria-describedby={status ? `settings-status-${path}` : undefined}
              className={({ isActive }) =>
                cn(
                  'flex flex-col gap-0.5 whitespace-nowrap px-3 py-2 text-sm font-medium transition-colors',
                  '-mb-px border-b-2 md:mb-0 md:rounded-md md:border-b-0',
                  isActive
                    ? 'border-primary text-foreground md:bg-accent'
                    : 'border-transparent text-muted-foreground hover:text-foreground md:hover:bg-muted/50',
                )
              }
            >
              <span>{label}</span>
              {status && (
                <span
                  id={`settings-status-${path}`}
                  className="hidden items-center gap-1.5 text-xs font-normal text-muted-foreground md:flex"
                >
                  {(status === 'Connected' || status.startsWith('On ·')) && (
                    <span aria-hidden="true" className="size-1.5 rounded-full bg-primary" />
                  )}
                  {status === "Can't reach Plex" && (
                    <span aria-hidden="true" className="size-1.5 rounded-full bg-destructive" />
                  )}
                  {status}
                </span>
              )}
            </NavLink>
          )
        })}
      </nav>
      <div className="min-w-0 flex-1">
        <Outlet />
      </div>
    </div>
  )
}
