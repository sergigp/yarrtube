import { createContext, useContext } from 'react'

/** The video copy whose categories are being edited. */
export interface EditedVideo {
  recordId: string
  title: string
  sourceName: string
}

/** Provided by `VideoCategoriesEditor`. */
export const EditVideoCategoriesContext = createContext<((video: EditedVideo) => void) | null>(
  null,
)

/** Opens the categories dialog for a video; needs a `VideoCategoriesEditor` above. */
export function useEditVideoCategories(): (video: EditedVideo) => void {
  const edit = useContext(EditVideoCategoriesContext)
  if (!edit) {
    throw new Error('useEditVideoCategories needs a VideoCategoriesEditor above it')
  }
  return edit
}
