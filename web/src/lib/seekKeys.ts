/** How far one arrow key press seeks, in seconds. */
export const SEEK_SECONDS = 10

/** How long the seek feedback stays up; presses within it add up. */
export const SEEK_FEEDBACK_MS = 700

export type SeekDirection = 'back' | 'forward'

/**
 * The direction a key press seeks the playing video, or null when it isn't
 * a seek: not a bare ←/→, already handled (e.g. by an open menu), or typed
 * into a field.
 */
export function seekDirection(event: KeyboardEvent): SeekDirection | null {
  if (event.defaultPrevented || event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) {
    return null
  }
  if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') {
    return null
  }
  if (isTextEntry(event.target)) {
    return null
  }
  return event.key === 'ArrowLeft' ? 'back' : 'forward'
}

function isTextEntry(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) {
    return false
  }
  return (
    target.isContentEditable ||
    target instanceof HTMLTextAreaElement ||
    target instanceof HTMLSelectElement ||
    (target instanceof HTMLInputElement && target.type !== 'checkbox' && target.type !== 'radio') ||
    target.getAttribute('role') === 'slider'
  )
}

/** `currentTime` moved `SEEK_SECONDS` towards `direction`, kept within the video. */
export function seekTime(currentTime: number, duration: number, direction: SeekDirection): number {
  const target = currentTime + (direction === 'back' ? -SEEK_SECONDS : SEEK_SECONDS)
  const end = Number.isFinite(duration) ? duration : Number.POSITIVE_INFINITY
  return Math.min(Math.max(target, 0), end)
}
