import { Link, useSearchParams } from 'react-router-dom'
import { useCategories } from '@/api/queries'
import type { Category } from '@/api/types'
import { memberCount } from '@/lib/categories'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'

function memberCountLabel(category: Category): string {
  const count = memberCount(category)
  return `${count} ${count === 1 ? 'member' : 'members'}`
}

interface CategoryListProps {
  categories: Category[]
  selectedId: string | null
}

function CategoryList({ categories, selectedId }: CategoryListProps) {
  return (
    <ul className="flex flex-col" aria-label="Category list">
      {categories.map((category) => {
        const selected = category.id === selectedId
        return (
          <li key={category.id}>
            <Link
              to={`/categories?selected=${encodeURIComponent(category.id)}`}
              aria-current={selected ? 'page' : undefined}
              className={cn(
                'flex items-baseline justify-between gap-2 rounded-md px-3 py-2 text-sm no-underline transition-colors hover:bg-secondary',
                selected ? 'bg-accent font-medium text-primary' : 'text-foreground',
              )}
            >
              <span className="min-w-0 truncate">{category.name}</span>
              <span className="shrink-0 text-xs text-muted-foreground">
                {memberCountLabel(category)}
              </span>
            </Link>
          </li>
        )
      })}
    </ul>
  )
}

function CategoryEditor({ category }: { category: Category }) {
  return (
    <section className="flex min-w-0 flex-col gap-6">
      <Input aria-label="Category name" defaultValue={category.name} key={category.id} />
    </section>
  )
}

/** The categories management view; the selected category is `?selected=<id>`. */
export function CategoriesView() {
  const [searchParams] = useSearchParams()
  const selectedId = searchParams.get('selected')
  const { data } = useCategories()
  const categories = data?.categories
  const selected = categories?.find((category) => category.id === selectedId) ?? null

  return (
    <div className="flex min-w-0 flex-col gap-6">
      <h1 className="font-heading text-xl font-semibold">Categories</h1>
      <div className="flex min-w-0 flex-col gap-6 md:flex-row">
        {categories && (
          <div className="md:w-64 md:shrink-0">
            <CategoryList categories={categories} selectedId={selectedId} />
          </div>
        )}
        {selected && <CategoryEditor category={selected} />}
      </div>
    </div>
  )
}
