interface SummariesFieldProps {
  id: string
  checked: boolean
  onChange: (checked: boolean) => void
  /** Called when the "AI settings" link is followed, to close the dialog. */
  onNavigateAway: () => void
}

/** The "Summarize videos" checkbox of the add and edit source dialogs. */
export function SummariesField(_props: SummariesFieldProps) {
  return null
}
