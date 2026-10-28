import { useState, type FormEvent } from 'react'
import { useNavigate } from 'react-router-dom'
import { createCategory } from '@/api/client'
import { useCategories, useInvalidateLibrary } from '@/api/queries'
import { defaultColor } from '@/lib/categoryColors'
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Button } from '@/components/ui/button'
import { CategoryColorPicker } from './CategoryColorPicker'
import type { ManageSourcesState } from './CategoryDetail'

interface AddCategoryDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

/** Asks for a new category's name and color, then opens its view to pick its sources. */
export function AddCategoryDialog({ open, onOpenChange }: AddCategoryDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="gap-5 p-6 sm:max-w-sm">
        <DialogTitle>Add category</DialogTitle>
        {/* The form unmounts with the dialog, so every open starts empty. */}
        <AddCategoryForm onClose={() => onOpenChange(false)} />
      </DialogContent>
    </Dialog>
  )
}

function AddCategoryForm({ onClose }: { onClose: () => void }) {
  const navigate = useNavigate()
  const invalidateLibrary = useInvalidateLibrary()
  const { data: categories } = useCategories()
  const [name, setName] = useState('')
  // Until the user picks one, the least used color, once categories load.
  const [chosenColor, setChosenColor] = useState<string | null>(null)
  const color =
    chosenColor ?? defaultColor((categories?.categories ?? []).map((category) => category.color))
  const [error, setError] = useState<Error | null>(null)
  const [submitting, setSubmitting] = useState(false)
  const canSubmit = name.trim() !== '' && !submitting

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (!canSubmit) {
      return
    }
    setSubmitting(true)
    setError(null)
    try {
      const category = await createCategory(name, color)
      invalidateLibrary()
      onClose()
      const state: ManageSourcesState = { manageSources: true }
      navigate(`/categories/${encodeURIComponent(category.id)}`, { state })
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)))
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <form className="flex flex-col gap-5" onSubmit={handleSubmit}>
      <div className="flex flex-col gap-1.5">
        <Label htmlFor="add-category-name">Name</Label>
        <Input
          id="add-category-name"
          type="text"
          value={name}
          onChange={(event) => setName(event.target.value)}
        />
      </div>
      <div className="flex flex-col gap-1.5">
        <span className="text-sm leading-none font-medium">Color</span>
        <CategoryColorPicker value={color} onChange={setChosenColor} />
      </div>
      {error && <p className="text-sm text-destructive">{error.message}</p>}
      <Button type="submit" disabled={!canSubmit} className="self-start">
        {submitting ? 'Creating…' : 'Create'}
      </Button>
    </form>
  )
}
