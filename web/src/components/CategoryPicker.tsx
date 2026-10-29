import { useId, useState, type KeyboardEvent } from 'react'
import { Check, Plus } from 'lucide-react'
import { createCategory } from '@/api/client'
import { useCategories, useInvalidateLibrary } from '@/api/queries'
import { byName } from '@/lib/categories'
import { errorMessage } from '@/lib/errorMessage'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { cn } from '@/lib/utils'

interface CategoryPickerProps {
  value: string[]
  onChange: (ids: string[]) => void
}

/**
 * Every category as a chip in its own color, toggled on or off for a source
 * with a click (chosen ones filled, the rest outlined), and a field to create
 * a further one, which is chosen at once. Typing an existing name there
 * chooses that category instead.
 */
export function CategoryPicker({ value, onChange }: CategoryPickerProps) {
  const { data } = useCategories()
  const invalidateLibrary = useInvalidateLibrary()
  const [text, setText] = useState('')
  const [error, setError] = useState<string | null>(null)
  const labelId = useId()
  const categories = byName(data?.categories)
  const typed = text.trim()

  const toggle = (id: string) =>
    onChange(value.includes(id) ? value.filter((chosen) => chosen !== id) : [...value, id])

  const create = async () => {
    if (typed === '') {
      return
    }
    setError(null)
    const existing = categories.find(
      (category) => category.name.toLowerCase() === typed.toLowerCase(),
    )
    if (existing) {
      if (!value.includes(existing.id)) {
        onChange([...value, existing.id])
      }
      setText('')
      return
    }
    try {
      const category = await createCategory(typed)
      await invalidateLibrary()
      onChange([...value, category.id])
      setText('')
    } catch (err) {
      setError(errorMessage(err))
    }
  }

  // Typing in the field never submits the dialog around it: Enter creates
  // the typed category, Escape clears the text.
  const handleKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'Enter') {
      event.preventDefault()
      void create()
    } else if (event.key === 'Escape' && text !== '') {
      event.preventDefault()
      event.stopPropagation()
      setText('')
    }
  }

  return (
    <div role="group" aria-labelledby={labelId} className="flex flex-col gap-1.5">
      <Label id={labelId}>Categories</Label>
      {categories.length > 0 && (
        <ul className="flex flex-wrap gap-1.5">
          {categories.map((category) => {
            const chosen = value.includes(category.id)
            return (
              <li key={category.id}>
                <button
                  type="button"
                  aria-pressed={chosen}
                  className={cn(
                    'flex items-center gap-1.5 rounded-full border py-1 pr-3 pl-2 text-xs font-medium transition-colors',
                    chosen
                      ? 'border-black/10 text-foreground'
                      : 'border-border text-muted-foreground hover:text-foreground',
                  )}
                  style={chosen ? { background: category.color } : undefined}
                  onClick={() => toggle(category.id)}
                >
                  {chosen ? (
                    <Check className="size-3" />
                  ) : (
                    <span
                      aria-hidden
                      className="size-3 rounded-full border border-black/10"
                      style={{ background: category.color }}
                    />
                  )}
                  {category.name}
                </button>
              </li>
            )
          })}
        </ul>
      )}
      <div className="flex gap-1.5">
        <Input
          aria-label="New category"
          placeholder="New category"
          className="h-8 text-sm"
          value={text}
          onChange={(event) => setText(event.target.value)}
          onKeyDown={handleKeyDown}
        />
        <Button
          type="button"
          variant="outline"
          disabled={typed === ''}
          onClick={() => void create()}
        >
          <Plus />
          Add
        </Button>
      </div>
      {error && <p className="text-sm text-destructive">{error}</p>}
    </div>
  )
}
