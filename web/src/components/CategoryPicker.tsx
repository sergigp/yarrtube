import { useId, useState } from 'react'
import { Plus, X } from 'lucide-react'
import { createCategory } from '@/api/client'
import { useCategories, useInvalidateLibrary } from '@/api/queries'
import { byName } from '@/lib/categories'
import { errorMessage } from '@/lib/errorMessage'
import { matchesSearch } from '@/lib/sidebarSections'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'

interface CategoryPickerProps {
  value: string[]
  onChange: (ids: string[]) => void
}

/**
 * The categories chosen for a source, as removable chips, and a field to pick
 * further ones by name, or create one when none matches what was typed.
 */
export function CategoryPicker({ value, onChange }: CategoryPickerProps) {
  const { data } = useCategories()
  const invalidateLibrary = useInvalidateLibrary()
  const [text, setText] = useState('')
  const [error, setError] = useState<string | null>(null)
  const fieldId = useId()
  const listboxId = useId()
  const categories = byName(data?.categories)
  const chosen = value.flatMap((id) => categories.filter((category) => category.id === id))
  const typed = text.trim()
  const options = typed
    ? categories.filter((category) => !value.includes(category.id) && matchesSearch(category, typed))
    : []
  const offersCreate =
    typed !== '' &&
    !categories.some((category) => category.name.toLowerCase() === typed.toLowerCase())

  const pick = (id: string) => {
    onChange([...value, id])
    setText('')
  }

  const create = async () => {
    setError(null)
    try {
      const category = await createCategory(typed)
      await invalidateLibrary()
      pick(category.id)
    } catch (err) {
      setError(errorMessage(err))
    }
  }

  return (
    <div className="flex flex-col gap-1.5">
      <Label htmlFor={fieldId}>Categories</Label>
      <ul aria-label="Chosen categories" className="flex flex-wrap gap-1.5 empty:hidden">
        {chosen.map((category) => (
          <li
            key={category.id}
            className="flex items-center gap-1 rounded-full bg-secondary py-0.5 pr-1 pl-2.5 text-xs font-medium"
          >
            {category.name}
            <button
              type="button"
              className="rounded-full p-0.5 text-muted-foreground transition-colors hover:text-foreground"
              aria-label={`Remove ${category.name}`}
              onClick={() => onChange(value.filter((id) => id !== category.id))}
            >
              <X className="size-3" />
            </button>
          </li>
        ))}
      </ul>
      <Input
        id={fieldId}
        role="combobox"
        aria-expanded={options.length > 0 || offersCreate}
        aria-controls={listboxId}
        aria-autocomplete="list"
        placeholder="Add a category"
        value={text}
        onChange={(event) => setText(event.target.value)}
      />
      {(options.length > 0 || offersCreate) && (
        <ul id={listboxId} role="listbox" className="flex flex-col rounded-md border border-border py-1">
          {options.map((category) => (
            <li
              key={category.id}
              role="option"
              aria-selected={false}
              className="cursor-pointer px-3 py-1.5 text-sm hover:bg-secondary"
              onClick={() => pick(category.id)}
            >
              {category.name}
            </li>
          ))}
          {offersCreate && (
            <li
              role="option"
              aria-selected={false}
              className="flex cursor-pointer items-center gap-1.5 px-3 py-1.5 text-sm hover:bg-secondary"
              onClick={create}
            >
              <Plus className="size-3.5" />
              {`Create "${typed}"`}
            </li>
          )}
        </ul>
      )}
      {error && <p className="text-sm text-destructive">{error}</p>}
    </div>
  )
}
