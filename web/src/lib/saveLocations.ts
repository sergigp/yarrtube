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
  void items
  void cap
  return [{ path: defaultParent, count: 0 }]
}

export function readRememberedParent(mode: SaveMode): string | null {
  void mode
  return null
}

export function writeRememberedParent(mode: SaveMode, parent: string): void {
  void mode
  void parent
}
