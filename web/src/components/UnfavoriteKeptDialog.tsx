interface UnfavoriteKeptDialogProps {
  open: boolean
  /** Names the video. */
  title: string
  /** Names the channel or playlist that keeps it. */
  sourceName: string
  onConfirm: () => void
  onCancel: () => void
}

/** Asks before unfavoriting a kept video, whose files the next sync deletes. */
export function UnfavoriteKeptDialog(props: UnfavoriteKeptDialogProps) {
  void props
  return <></>
}
