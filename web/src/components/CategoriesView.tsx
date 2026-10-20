import { useState } from 'react'
import { Link, useSearchParams } from 'react-router-dom'
import { renameCategory } from '@/api/client'
import { useCategories, useInvalidateLibrary } from '@/api/queries'
import type { Category } from '@/api/types'
import { memberCount } from '@/lib/categories'
import { errorMessage } from '@/lib/errorMessage'
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

/**
 * The category's name, saved when the field loses focus or Enter is pressed.
 * A rejected name shows why and goes back to the saved one.
 */
function CategoryNameField({ category }: { category: Category }) {
  const invalidateLibrary = useInvalidateLibrary()
  const [name, setName] = useState(category.name)
  const [error, setError] = useState<string | null>(null)

  const save = async () => {
    if (name === category.name) {
      return
    }
    setError(null)
    try {
      await renameCategory(category.id, name)
      await invalidateLibrary()
    } catch (err) {
      setError(errorMessage(err))
      setName(category.name)
    }
  }

  return (
    <div className="flex flex-col gap-1.5">
      <Input
        aria-label="Category name"
        value={name}
        onChange={(event) => setName(event.target.value)}
        onBlur={save}
        onKeyDown={(event) => {
          if (event.key === 'Enter') {
            event.preventDefault()
            void save()
          }
        }}
        className="font-heading text-lg font-semibold"
      />
      {error && <p className="text-sm text-destructive">{error}</p>}
    </div>
  )
}

function CategoryEditor({ category }: { category: Category }) {
  return (
    <section className="flex min-w-0 flex-1 flex-col gap-6">
      <CategoryNameField key={category.id} category={category} />
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
