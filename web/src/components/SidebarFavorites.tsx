import { Link } from 'react-router-dom'
import { Star } from 'lucide-react'
import { cn } from '@/lib/utils'
import { SIDEBAR_ROW_CLASS, sidebarRowLinkClassName } from './SidebarRowParts'

/** The `/favorites` route the sidebar's Favorites row opens. */
export const FAVORITES_PATH = '/favorites'

/**
 * The sidebar's "Favorites" row, opening the Favorites view; shown whether
 * or not any favorite exists, and marked active while the user is there.
 */
export function SidebarFavorites({
  active,
  onNavigate,
}: {
  active: boolean
  onNavigate: () => void
}) {
  return (
    <section>
      <ul>
        <li className={cn(SIDEBAR_ROW_CLASS, active && 'bg-accent')}>
          <Link
            to={FAVORITES_PATH}
            onClick={onNavigate}
            aria-current={active ? 'page' : undefined}
            className={sidebarRowLinkClassName(active)}
          >
            <Star aria-hidden="true" className="size-4 shrink-0" />
            <span className="min-w-0 truncate">Favorites</span>
          </Link>
        </li>
      </ul>
    </section>
  )
}
