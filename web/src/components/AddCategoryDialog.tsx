import { useState, type FormEvent } from 'react'
import { useNavigate } from 'react-router-dom'
import { createCategory } from '@/api/client'
import { useInvalidateLibrary } from '@/api/queries'
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Button } from '@/components/ui/button'
import type { ManageSourcesState } from './CategoryDetail'

interface AddCategoryDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

/** Asks for a new category's name, then opens its view to pick its sources. */
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
  const [name, setName] = useState('')
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
      const category = await createCategory(name)
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
      {error && <p className="text-sm text-destructive">{error.message}</p>}
      <Button type="submit" disabled={!canSubmit} className="self-start">
        {submitting ? 'Creating…' : 'Create'}
      </Button>
    </form>
  )
}
