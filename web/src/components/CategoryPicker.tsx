import { useId, useState, type KeyboardEvent } from 'react'
import { Plus, X } from 'lucide-react'
import { createCategory } from '@/api/client'
import { useCategories, useInvalidateLibrary } from '@/api/queries'
import { byName } from '@/lib/categories'
import { errorMessage } from '@/lib/errorMessage'
import { matchesSearch } from '@/lib/sidebarSections'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { cn } from '@/lib/utils'

/** The listbox entry that creates a category from the typed name. */
const CREATE_KEY = 'create'

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
  const [highlighted, setHighlighted] = useState(0)
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
    changeText('')
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

  // What the listbox offers, in order: the matching categories, then creating one.
  const choices = [
    ...options.map((category) => ({
      key: category.id,
      label: category.name,
      choose: () => pick(category.id),
    })),
    ...(offersCreate
      ? [{ key: CREATE_KEY, label: `Create "${typed}"`, choose: () => void create() }]
      : []),
  ]
  const active = Math.min(highlighted, choices.length - 1)
  const optionId = (index: number) => `${listboxId}-${index}`

  function changeText(next: string) {
    setText(next)
    setHighlighted(0)
  }

  // Typing in the field never submits the dialog around it: Enter chooses
  // the highlighted entry, arrows move the highlight, Escape clears the text.
  const handleKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault()
      const step = event.key === 'ArrowDown' ? 1 : -1
      setHighlighted((active + step + choices.length) % Math.max(choices.length, 1))
    } else if (event.key === 'Enter' && typed !== '') {
      event.preventDefault()
      choices[active]?.choose()
    } else if (event.key === 'Escape' && text !== '') {
      event.preventDefault()
      event.stopPropagation()
      changeText('')
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
        aria-expanded={choices.length > 0}
        aria-controls={listboxId}
        aria-autocomplete="list"
        aria-activedescendant={choices.length > 0 ? optionId(active) : undefined}
        placeholder="Add a category"
        value={text}
        onChange={(event) => changeText(event.target.value)}
        onKeyDown={handleKeyDown}
      />
      {choices.length > 0 && (
        <ul id={listboxId} role="listbox" className="flex flex-col rounded-md border border-border py-1">
          {choices.map((choice, index) => (
            <li
              key={choice.key}
              id={optionId(index)}
              role="option"
              aria-selected={index === active}
              className={cn(
                'flex cursor-pointer items-center gap-1.5 px-3 py-1.5 text-sm hover:bg-secondary',
                index === active && 'bg-secondary',
              )}
              onMouseEnter={() => setHighlighted(index)}
              onClick={choice.choose}
            >
              {choice.key === CREATE_KEY && <Plus className="size-3.5" />}
              {choice.label}
            </li>
          ))}
        </ul>
      )}
      {error && <p className="text-sm text-destructive">{error}</p>}
    </div>
  )
}
