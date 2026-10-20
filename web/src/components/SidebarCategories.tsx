import { Link, useLocation } from 'react-router-dom'
import { Plus, Settings2 } from 'lucide-react'
import { useCategories } from '@/api/queries'
import type { Category } from '@/api/types'
import { useRememberedFlag } from '@/hooks/useRememberedFlag'
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
export function SidebarCategories({ onAddCategory, onClose }: SidebarCategoriesProps) {
  const { pathname } = useLocation()
  const activeId = activeIdFrom(pathname, '/categories/')
  const { data } = useCategories()
  const categories = data && orderCategories(data.categories)
  const [folded, toggleFolded] = useRememberedFlag('yarrtube.sidebar.folded.categories', false)
  const headingActive = folded && (categories?.some((category) => category.id === activeId) ?? false)

  return (
    <div>
      <SidebarSectionHeading
        title="Categories"
        folded={folded}
        onToggleFolded={toggleFolded}
        headingActive={headingActive}
        foldedTotal={data?.unwatched_total}
        actions={
          <Link
            to="/categories"
            onClick={onClose}
            aria-label="Manage categories"
            title="Manage categories"
            className="flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground"
          >
            <Settings2 className="size-4" />
          </Link>
        }
      />
      {!folded && (
        <>
          {/* Closing first lets the mobile drawer get out of the dialog's way. */}
          <button
            type="button"
            className="mb-1 flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground"
            onClick={() => {
              onClose()
              onAddCategory()
            }}
          >
            <Plus className="size-4 shrink-0" />
            Add category
          </button>
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
        </>
      )}
    </div>
  )
}
