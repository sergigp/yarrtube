import { useId, useState } from 'react'
import { Link, useLocation, useNavigate, useSearchParams } from 'react-router-dom'
import { ChevronLeft, Plus, Trash2 } from 'lucide-react'
import {
  avatarMediaUrl,
  deleteCategory,
  renameCategory,
  setCategoryMembers,
} from '@/api/client'
import {
  useCategories,
  useChannels,
  useInvalidateLibrary,
  useLibraryAction,
  usePlaylists,
} from '@/api/queries'
import type { Category, CategoryMembers, LibraryItem } from '@/api/types'
import {
  byName,
  deleteCategoryDescription,
  isMember,
  memberCount,
  withMembers,
  withoutMember,
  type SourceRef,
} from '@/lib/categories'
import { matchesSearch } from '@/lib/sidebarSections'
import { useIsDesktop } from '@/hooks/useIsDesktop'
import { errorMessage } from '@/lib/errorMessage'
import { AddCategoryDialog, type ManageCategoryState } from './AddCategoryDialog'
import { ConfirmDialog } from './ConfirmDialog'
import { Thumbnail } from './Thumbnail'
import { Button } from '@/components/ui/button'
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

interface MemberGroupProps {
  title: string
  items: LibraryItem[]
  showAvatar?: boolean
  disabled: boolean
  onRemove: (item: LibraryItem) => void
}

function MemberGroup({ title, items, showAvatar, disabled, onRemove }: MemberGroupProps) {
  const headingId = useId()
  return (
    <div className="flex flex-col gap-1">
      <h3 id={headingId} className="text-sm font-medium text-muted-foreground">
        {title}
      </h3>
      <ul aria-labelledby={headingId} className="flex flex-col">
        {items.map((item) => (
          <li key={item.id} className="flex items-center gap-2 rounded-md px-2 py-1 text-sm">
            <span className="flex min-w-0 flex-1 items-center gap-2">
              {showAvatar && (
                <Thumbnail
                  src={item.avatar_filename ? avatarMediaUrl(item.avatar_filename) : null}
                  className="size-5 shrink-0 rounded-full object-cover"
                />
              )}
              <span className="truncate">{item.name}</span>
            </span>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              aria-label={`Remove ${item.name}`}
              disabled={disabled}
              onClick={() => onRemove(item)}
            >
              Remove
            </Button>
          </li>
        ))}
      </ul>
    </div>
  )
}

/** The category's members, by kind, each removable at once. */
/**
 * Saves a category's members, one change at a time: `saving` holds until the
 * change is saved and the refetched category shows it, so the next change is
 * always built from the members as they are.
 */
function useMemberChanges(category: Category) {
  const invalidateLibrary = useInvalidateLibrary()
  const [saving, setSaving] = useState(false)

  const save = async (members: CategoryMembers, failure: string): Promise<boolean> => {
    setSaving(true)
    try {
      await setCategoryMembers(category.id, members)
      await invalidateLibrary()
      return true
    } catch (err) {
      window.alert(`${failure}: ${errorMessage(err)}`)
      return false
    } finally {
      setSaving(false)
    }
  }

  return { saving, save }
}

type MemberChanges = ReturnType<typeof useMemberChanges>

function CategoryMemberList({ category, changes }: { category: Category; changes: MemberChanges }) {
  const { data: channels } = useChannels()
  const { data: playlists } = usePlaylists()
  const headingId = useId()

  const remove = (source: SourceRef) =>
    void changes.save(
      withoutMember(category, source),
      `Failed to remove it from "${category.name}"`,
    )

  return (
    <section aria-labelledby={headingId} className="flex flex-col gap-3">
      <h2 id={headingId} className="font-heading text-base font-semibold">
        In {category.name}
      </h2>
      {memberCount(category) === 0 && (
        <p className="text-sm text-muted-foreground">
          Nothing here yet. Pick channels and playlists below to add them.
        </p>
      )}
      {category.channels.length > 0 && (
        <MemberGroup
          title="Channels"
          items={byName(channels?.filter((channel) => category.channels.includes(channel.id)))}
          showAvatar
          disabled={changes.saving}
          onRemove={(channel) => remove({ kind: 'channel', id: channel.id })}
        />
      )}
      {category.playlists.length > 0 && (
        <MemberGroup
          title="Playlists"
          items={byName(playlists?.filter((playlist) => category.playlists.includes(playlist.id)))}
          disabled={changes.saving}
          onRemove={(playlist) => remove({ kind: 'playlist', id: playlist.id })}
        />
      )}
    </section>
  )
}

/**
 * The tracked channels and playlists not in the category, filtered by name,
 * to check and add at once.
 */
function AddMembersPicker({ category, changes }: { category: Category; changes: MemberChanges }) {
  const { data: channels } = useChannels()
  const { data: playlists } = usePlaylists()
  const location = useLocation()
  const focusSearch = (location.state as ManageCategoryState | null)?.focusAddMembers ?? false
  const [search, setSearch] = useState('')
  const [checked, setChecked] = useState<SourceRef[]>([])
  const text = search.trim()
  const candidates = (items: LibraryItem[] | undefined, kind: SourceRef['kind']) =>
    byName(items)
      .filter((item) => !isMember(category, { kind, id: item.id }))
      .filter((item) => !text || matchesSearch(item, text))
      .map((item) => ({ item, source: { kind, id: item.id } }))
  const pickable = [...candidates(channels, 'channel'), ...candidates(playlists, 'playlist')]
  const isChecked = (source: SourceRef) =>
    checked.some((other) => other.kind === source.kind && other.id === source.id)
  const toggle = (source: SourceRef) =>
    setChecked((prev) =>
      isChecked(source)
        ? prev.filter((other) => other.kind !== source.kind || other.id !== source.id)
        : [...prev, source],
    )

  const addSelected = async () => {
    const added = await changes.save(
      withMembers(category, checked),
      `Failed to add to "${category.name}"`,
    )
    if (added) {
      setChecked([])
    }
  }

  return (
    <fieldset className="flex min-w-0 flex-col gap-2">
      <legend className="mb-2 font-heading text-base font-semibold">Add to {category.name}</legend>
      <Input
        type="search"
        aria-label="Search channels and playlists to add"
        placeholder="Search channels and playlists"
        value={search}
        onChange={(event) => setSearch(event.target.value)}
        autoFocus={focusSearch}
      />
      {channels?.length === 0 && playlists?.length === 0 && (
        <p className="text-sm text-muted-foreground">
          No channels or playlists tracked yet. Add one from the sidebar, then come back to group
          it here.
        </p>
      )}
      <ul className="flex max-h-72 flex-col overflow-y-auto">
        {pickable.map(({ item, source }) => (
          <li key={`${source.kind}:${source.id}`}>
            <label className="flex items-center gap-2 rounded-md px-2 py-1 text-sm hover:bg-secondary">
              <input
                type="checkbox"
                className="size-4 shrink-0 accent-primary"
                checked={isChecked(source)}
                onChange={() => toggle(source)}
              />
              {item.name}
            </label>
          </li>
        ))}
      </ul>
      <Button
        type="button"
        className="self-start"
        disabled={checked.length === 0 || changes.saving}
        onClick={addSelected}
      >
        Add selected
      </Button>
    </fieldset>
  )
}

/** Deletes the category after confirming that its sources and videos stay. */
function DeleteCategoryButton({ category }: { category: Category }) {
  const navigate = useNavigate()
  const refreshing = useLibraryAction()
  const [confirming, setConfirming] = useState(false)

  return (
    <>
      <Button
        type="button"
        variant="destructive"
        className="self-start"
        onClick={() => setConfirming(true)}
      >
        <Trash2 />
        Delete category
      </Button>
      <ConfirmDialog
        open={confirming}
        onOpenChange={setConfirming}
        title={`Delete "${category.name}"?`}
        description={deleteCategoryDescription(category)}
        onConfirm={refreshing(async () => {
          await deleteCategory(category.id)
          navigate('/categories')
        })}
      />
    </>
  )
}

function CategoryEditor({ category, showBack }: { category: Category; showBack: boolean }) {
  const changes = useMemberChanges(category)
  return (
    <section className="flex min-w-0 flex-1 flex-col gap-6">
      {showBack && (
        <Link
          to="/categories"
          className="flex items-center gap-1 self-start text-sm text-muted-foreground no-underline hover:text-foreground"
        >
          <ChevronLeft className="size-4" />
          Back to categories
        </Link>
      )}
      <CategoryNameField key={category.id} category={category} />
      <Link
        to={`/categories/${encodeURIComponent(category.id)}`}
        className="self-start text-sm text-primary no-underline hover:underline"
      >
        Open {category.name}
      </Link>
      <CategoryMemberList category={category} changes={changes} />
      <AddMembersPicker key={category.id} category={category} changes={changes} />
      <DeleteCategoryButton category={category} />
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
  const desktop = useIsDesktop()
  // Below the desktop breakpoint, the selected category's details replace the list.
  const showList = desktop || !selected
  const [adding, setAdding] = useState(false)

  return (
    <div className="flex min-w-0 flex-col gap-6">
      <div className="flex items-center justify-between gap-4">
        <h1 className="font-heading text-xl font-semibold">Categories</h1>
        <Button type="button" variant="outline" size="sm" onClick={() => setAdding(true)}>
          <Plus />
          New category
        </Button>
      </div>
      {categories && categories.length === 0 && (
        <p className="text-sm text-muted-foreground">
          No categories yet. Create one to group channels and playlists.
        </p>
      )}
      <div className="flex min-w-0 flex-col gap-6 md:flex-row">
        {categories && showList && (
          <div className="md:w-64 md:shrink-0">
            <CategoryList categories={categories} selectedId={selectedId} />
          </div>
        )}
        {selected && <CategoryEditor category={selected} showBack={!desktop} />}
      </div>
      <AddCategoryDialog open={adding} onOpenChange={setAdding} />
    </div>
  )
}
