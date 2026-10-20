import { useState } from 'react'
import { Link, useLocation, useNavigate } from 'react-router-dom'
import { Ellipsis, Plus, Settings2 } from 'lucide-react'
import { deleteCategory } from '@/api/client'
import { useCategories, useLibraryAction } from '@/api/queries'
import type { Category } from '@/api/types'
import { useRememberedFlag } from '@/hooks/useRememberedFlag'
import { deleteCategoryDescription, orderCategories } from '@/lib/categories'
import { activeIdFrom } from '@/lib/sidebarSections'
import { cn } from '@/lib/utils'
import type { ManageSourcesState } from './CategoryDetail'
import { ConfirmDialog } from './ConfirmDialog'
import { EntryActionsMenu } from './EntryActionsMenu'
import { SidebarSectionHeading } from './SidebarSectionHeading'
import { DropdownMenuItem } from '@/components/ui/dropdown-menu'

interface CategoryRowProps {
  category: Category
  active: boolean
  onNavigate: () => void
  onDeleteRequest: () => void
}

function CategoryRow({ category, active, onNavigate, onDeleteRequest }: CategoryRowProps) {
  const navigate = useNavigate()
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
      <EntryActionsMenu
        name={category.name}
        triggerIcon={<Ellipsis className="size-4" />}
        leadingItems={
          <DropdownMenuItem
            onSelect={() => {
              onNavigate()
              const state: ManageSourcesState = { manageSources: true }
              navigate(`/categories/${encodeURIComponent(category.id)}`, { state })
            }}
          >
            <Settings2 />
            Manage sources
          </DropdownMenuItem>
        }
        onDeleteRequest={onDeleteRequest}
      />
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
  const navigate = useNavigate()
  const { data } = useCategories()
  const refreshing = useLibraryAction()
  const [pendingDelete, setPendingDelete] = useState<Category | null>(null)
  const categories = data && orderCategories(data.categories)
  const [folded, toggleFolded] = useRememberedFlag('yarrtube.sidebar.folded.categories', false)
  const headingActive = folded && (categories?.some((category) => category.id === activeId) ?? false)
  // Closing first lets the mobile drawer get out of the dialog's way.
  const add = () => {
    onClose()
    onAddCategory()
  }
  // No categories yet gets a full-width add row; otherwise a compact `+` on the heading.
  const empty = categories?.length === 0

  return (
    <div>
      <SidebarSectionHeading
        title="Categories"
        folded={folded}
        onToggleFolded={toggleFolded}
        headingActive={headingActive}
        foldedTotal={data?.unwatched_total}
        add={empty ? undefined : { label: 'Add category', onClick: add }}
      />
      {!folded && (
        <>
          {empty && (
            <button
              type="button"
              className="mb-1 flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground"
              onClick={add}
            >
              <Plus className="size-4 shrink-0" />
              Add category
            </button>
          )}
          {categories && categories.length > 0 && (
            <ul className="flex flex-col" aria-label="Categories">
              {categories.map((category) => (
                <CategoryRow
                  key={category.id}
                  category={category}
                  active={category.id === activeId}
                  onNavigate={onClose}
                  onDeleteRequest={() => setPendingDelete(category)}
                />
              ))}
            </ul>
          )}
        </>
      )}
      <ConfirmDialog
        open={pendingDelete !== null}
        onOpenChange={(open) => {
          if (!open) {
            setPendingDelete(null)
          }
        }}
        title={pendingDelete ? `Delete "${pendingDelete.name}"?` : ''}
        description={pendingDelete ? deleteCategoryDescription(pendingDelete) : ''}
        onConfirm={refreshing(async () => {
          if (!pendingDelete) {
            return
          }
          await deleteCategory(pendingDelete.id)
          if (activeId === pendingDelete.id) {
            navigate('/')
          }
        })}
      />
    </div>
  )
}
