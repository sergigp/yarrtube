/** Moves `element` to `seconds` and plays it. */
export function playFrom(element: HTMLVideoElement, seconds: number): void {
  element.currentTime = seconds
  // A rejected play (e.g. autoplay policy) leaves it paused there.
  element.play().catch(() => {})
}
