interface VideoCategoriesDialogProps {
  /** The stored copy's `record_id`. */
  recordId: string
  /** The video's title, the dialog's title. */
  title: string
  /** The channel or playlist the copy comes from, named on locked chips. */
  sourceName: string
  open: boolean
  onOpenChange: (open: boolean) => void
}

/** Every category as a chip, set by hand for one video copy. */
export function VideoCategoriesDialog(_props: VideoCategoriesDialogProps) {
  return null
}
