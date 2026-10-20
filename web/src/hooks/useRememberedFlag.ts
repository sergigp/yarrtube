/** Storage unreadable or unwritable: starts at `initial`, lasts until reload. */
export function useRememberedFlag(key: string, initial: boolean): readonly [boolean, () => void] {
  void key
  return [initial, () => {}] as const
}
