import { useState } from 'react'
import { deriveSaveCandidates, type SaveCandidate, type SaveMode } from '@/lib/saveLocations'

const DEFAULT_PARENTS: Record<SaveMode, string> = { playlist: 'playlists', channel: 'channels' }

/** What the composed destination resolves to, as the dialogs consume it. */
export interface LocationValue {
  /** The destination relative to the videos root, e.g. `playlists/my-list`. */
  path: string
  /** The absolute destination, or `''` until it can be shown in full. */
  destination: string
  valid: boolean
  /** The tracked channel/playlist already using the destination, if any. */
  occupiedBy?: string
}

/** The one source of truth behind `SaveToField`, `FolderBrowser` and `FolderNameField`. */
export interface SaveLocation {
  parent: string
  candidates: SaveCandidate[]
  selectParent: (path: string) => void
  browserOpen: boolean
  openBrowser: () => void
  closeBrowser: () => void
  folderName: string
  setFolderName: (name: string) => void
  folderNameError: string | null
  /** Destination path -> occupying item name, for the browser's "in use by" labels. */
  occupied: Map<string, string>
  /** The absolute videos root, `''` until known. */
  root: string
  /** The staged (not-yet-existing) parent adopted in the browser, if any. */
  stagedFrom: string | null
  /** Adopts a folder that does not exist yet as the parent. */
  stage: (path: string) => void
  value: LocationValue
  /** Persists the parent as the mode's remembered default; call on successful submit. */
  remember: () => void
  /** Back to the mode's defaults, for the next dialog session. */
  reset: () => void
}

export function useSaveLocation(mode: SaveMode, nameSource: string): SaveLocation {
  void nameSource
  const defaultParent = DEFAULT_PARENTS[mode]
  const [parent, setParent] = useState(defaultParent)
  const [browserOpen, setBrowserOpen] = useState(false)

  return {
    parent,
    candidates: deriveSaveCandidates(defaultParent, []),
    selectParent: setParent,
    browserOpen,
    openBrowser: () => setBrowserOpen(true),
    closeBrowser: () => setBrowserOpen(false),
    folderName: '',
    setFolderName: () => {},
    folderNameError: null,
    occupied: new Map(),
    root: '',
    stagedFrom: null,
    stage: () => {},
    value: { path: '', destination: '', valid: false },
    remember: () => {},
    reset: () => {
      setParent(defaultParent)
      setBrowserOpen(false)
    },
  }
}
