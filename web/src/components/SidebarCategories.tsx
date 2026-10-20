import { Link } from 'react-router-dom'
import { useCategories } from '@/api/queries'
import type { Category } from '@/api/types'
import { orderCategories } from '@/lib/categories'
import { SidebarSectionHeading } from './SidebarSectionHeading'

interface CategoryRowProps {
  category: Category
  onNavigate: () => void
}

function CategoryRow({ category, onNavigate }: CategoryRowProps) {
  const unwatchedCount = category.unwatched_count
  return (
    <li className="flex items-center gap-1 rounded-md pr-1">
      <Link
        to={`/categories/${encodeURIComponent(category.id)}`}
        onClick={onNavigate}
        className="flex min-w-0 flex-1 items-center gap-2 truncate px-2 py-1.5 text-sm text-foreground no-underline transition-colors hover:text-primary"
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
            <CategoryRow key={category.id} category={category} onNavigate={onClose} />
          ))}
        </ul>
      )}
    </div>
  )
}
