import type { Category } from '@/api/types'
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog'

interface ManageSourcesDialogProps {
  category: Category
  open: boolean
  onOpenChange: (open: boolean) => void
}

/** Every tracked channel and playlist as a chip, pressed when in the category. */
export function ManageSourcesDialog({ category, open, onOpenChange }: ManageSourcesDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogTitle>Manage sources in {category.name}</DialogTitle>
      </DialogContent>
    </Dialog>
  )
}
