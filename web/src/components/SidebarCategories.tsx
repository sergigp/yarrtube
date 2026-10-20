import { Link, useLocation } from 'react-router-dom'
import { useCategories } from '@/api/queries'
import type { Category } from '@/api/types'
import { orderCategories } from '@/lib/categories'
import { activeIdFrom } from '@/lib/sidebarSections'
import { cn } from '@/lib/utils'
import { SidebarSectionHeading } from './SidebarSectionHeading'

interface CategoryRowProps {
  category: Category
  active: boolean
  onNavigate: () => void
}

function CategoryRow({ category, active, onNavigate }: CategoryRowProps) {
  const unwatchedCount = category.unwatched_count
  return (
    <li className={cn('flex items-center gap-1 rounded-md pr-1', active && 'bg-accent')}>
      <Link
        to={`/categories/${encodeURIComponent(category.id)}`}
        onClick={onNavigate}
        aria-current={active ? 'page' : undefined}
        className={cn(
          'flex min-w-0 flex-1 items-center gap-2 truncate px-2 py-1.5 text-sm no-underline transition-colors',
          active ? 'font-medium text-primary' : 'text-foreground hover:text-primary',
        )}
      >
        <span className="min-w-0 flex-1 truncate">{category.name}</span>
        {unwatchedCount > 0 && (
          <span
            className="shrink-0 rounded-full bg-primary px-1.5 py-0.5 text-[10px] leading-none font-medium text-primary-foreground"
            aria-label={`${unwatchedCount} unwatched`}
            title={`${unwatchedCount} unwatched`}
          >
            {unwatchedCount}
          </span>
        )}
      </Link>
    </li>
  )
}

interface SidebarCategoriesProps {
  onAddCategory: () => void
  onClose: () => void
}

/** The sidebar's Categories section: every category, unread first. */
export function SidebarCategories({ onClose }: SidebarCategoriesProps) {
  const { pathname } = useLocation()
  const activeId = activeIdFrom(pathname, '/categories/')
  const { data } = useCategories()
  const categories = data && orderCategories(data.categories)

  return (
    <div>
      <SidebarSectionHeading
        title="Categories"
        folded={false}
        onToggleFolded={() => {}}
        headingActive={false}
      />
      {categories && categories.length > 0 && (
        <ul className="flex flex-col" aria-label="Categories">
          {categories.map((category) => (
            <CategoryRow
              key={category.id}
              category={category}
              active={category.id === activeId}
              onNavigate={onClose}
            />
          ))}
        </ul>
      )}
    </div>
  )
}
