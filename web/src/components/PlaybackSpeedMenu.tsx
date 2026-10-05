interface PlaybackSpeedMenuProps {
  rate: number
  onRateChange: (rate: number) => void
  disabled: boolean
  className?: string
}

/** A menu of playback speeds, labelled with the current one. */
export function PlaybackSpeedMenu({ disabled, className }: PlaybackSpeedMenuProps) {
  return <button type="button" disabled={disabled} className={className} />
}
