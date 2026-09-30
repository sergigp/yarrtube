// Suggested "Save to" parent folders for the add dialogs, derived from the
// tracked items' storage paths, plus the per-mode remembered parent.

export type SaveMode = 'playlist' | 'channel'

export interface SaveCandidate {
  /** The parent folder relative to the videos root, e.g. `playlists/kids`. */
  path: string
  /** Tracked items of the dialog's kind stored directly under it. */
  count: number
}

const DEFAULT_CAP = 5

/** ISO 8601 timestamps compare correctly as strings; absent ones sort last. */
function byRecency(
  a: { newest?: string; count: number; path: string },
  b: { newest?: string; count: number; path: string },
): number {
  if (a.newest !== b.newest) {
    if (!a.newest) {
      return 1
    }
    if (!b.newest) {
      return -1
    }
    return a.newest > b.newest ? -1 : 1
  }
  return b.count - a.count || a.path.localeCompare(b.path)
}

export function deriveSaveCandidates(
  defaultParent: string,
  items: { path: string; created_at?: string }[],
  cap: number = DEFAULT_CAP,
): SaveCandidate[] {
  const groups = new Map<string, { count: number; newest?: string }>()
  for (const item of items) {
    const cut = item.path.lastIndexOf('/')
    // An item stored directly at the videos root has no parent to suggest.
    if (cut <= 0) {
      continue
    }
    const parent = item.path.slice(0, cut)
    const group = groups.get(parent) ?? { count: 0 }
    group.count += 1
    if (item.created_at && (!group.newest || item.created_at > group.newest)) {
      group.newest = item.created_at
    }
    groups.set(parent, group)
  }
  const derived = [...groups.entries()]
    .filter(([path]) => path !== defaultParent)
    .map(([path, { count, newest }]) => ({ path, count, newest }))
    .sort(byRecency)
    .map(({ path, count }) => ({ path, count }))
  const first = { path: defaultParent, count: groups.get(defaultParent)?.count ?? 0 }
  return [first, ...derived].slice(0, cap)
}

const STORAGE_PREFIX = 'yarrtube.save-to.'

export function readRememberedParent(mode: SaveMode): string | null {
  try {
    return window.localStorage.getItem(STORAGE_PREFIX + mode)
  } catch {
    // Unreadable storage (private windows, blocked site data) means nothing
    // remembered, never an error.
    return null
  }
}

export function writeRememberedParent(mode: SaveMode, parent: string): void {
  try {
    window.localStorage.setItem(STORAGE_PREFIX + mode, parent)
  } catch {
    // Unwritable storage loses the convenience, never the submit.
  }
}
