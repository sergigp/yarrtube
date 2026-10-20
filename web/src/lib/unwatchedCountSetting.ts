/** Describes changing a playlist's unwatched count to `hidden`, e.g. for a failure alert. */
export function unwatchedCountSettingAction(hidden: boolean, name: string): string {
  return `${hidden ? 'hide' : 'show'} the unwatched count of "${name}"`
}
