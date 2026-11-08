import { useState, type ReactNode } from 'react'
import { VideoCategoriesDialog } from '@/components/VideoCategoriesDialog'
import { EditVideoCategoriesContext, type EditedVideo } from '@/hooks/useEditVideoCategories'

/**
 * Hosts the one "Edit categories…" dialog, above every view: the card that
 * opened it can leave its list (taken out of the category shown) without
 * closing it.
 */
export function VideoCategoriesEditor({ children }: { children: ReactNode }) {
  const [edited, setEdited] = useState<EditedVideo | null>(null)
  return (
    <EditVideoCategoriesContext.Provider value={setEdited}>
      {children}
      {edited && (
        <VideoCategoriesDialog
          key={edited.recordId}
          recordId={edited.recordId}
          title={edited.title}
          sourceName={edited.sourceName}
          open
          onOpenChange={(open) => !open && setEdited(null)}
        />
      )}
    </EditVideoCategoriesContext.Provider>
  )
}
