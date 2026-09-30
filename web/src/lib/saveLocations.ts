// Suggested "Save to" parent folders for the add dialogs, derived from the
// tracked items' storage paths, plus the per-mode remembered parent.

export type SaveMode = 'playlist' | 'channel'

export interface SaveCandidate {
  /** The parent folder relative to the videos root, e.g. `playlists/kids`. */
  path: string
  /** Tracked items of the dialog's kind stored directly under it. */
  count: number
}

export function deriveSaveCandidates(
  defaultParent: string,
  items: { path: string; created_at?: string }[],
  cap?: number,
): SaveCandidate[] {
  void cap
  const groups = new Map<string, number>()
  for (const item of items) {
    const cut = item.path.lastIndexOf('/')
    // An item stored directly at the videos root has no parent to suggest.
    if (cut <= 0) {
      continue
    }
    const parent = item.path.slice(0, cut)
    groups.set(parent, (groups.get(parent) ?? 0) + 1)
  }
  const derived = [...groups.entries()]
    .filter(([path]) => path !== defaultParent)
    .map(([path, count]) => ({ path, count }))
  return [{ path: defaultParent, count: groups.get(defaultParent) ?? 0 }, ...derived]
}

export function readRememberedParent(mode: SaveMode): string | null {
  void mode
  return null
}

export function writeRememberedParent(mode: SaveMode, parent: string): void {
  void mode
  void parent
}
