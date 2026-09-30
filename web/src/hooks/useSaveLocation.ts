import { useCallback, useEffect, useMemo, useState } from 'react'
import { fetchDirectories } from '@/api/client'
import { useChannels, usePlaylists } from '@/api/queries'
import {
  deriveSaveCandidates,
  readRememberedParent,
  writeRememberedParent,
  type SaveCandidate,
  type SaveMode,
} from '@/lib/saveLocations'
import { slugify } from '@/lib/slugify'

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
}

function join(parent: string, name: string): string {
  return parent ? `${parent}/${name}` : name
}

function parentOf(path: string): string | null {
  const cut = path.lastIndexOf('/')
  return cut > 0 ? path.slice(0, cut) : null
}

/**
 * Whether `path` is the staged parent or sits below it. A staged parent does
 * not exist on disk, so neither it nor anything under it can be listed.
 */
function isStaged(path: string, stagedFrom: string | null): boolean {
  return Boolean(stagedFrom) && (path === stagedFrom || path.startsWith(`${stagedFrom}/`))
}

/**
 * Owns the storage-location state of an add dialog: the selected parent (fed
 * by the "Save to" candidates and the folder browser) and the folder name
 * (auto-filled from `nameSource` until edited by hand), composed into the
 * destination the dialog's notice shows and its create request submits.
 *
 * Meant to live in a component that unmounts when the dialog closes, so each
 * dialog session starts fresh.
 */
export function useSaveLocation(mode: SaveMode, nameSource: string): SaveLocation {
  const defaultParent = DEFAULT_PARENTS[mode]
  const [parent, setParent] = useState(defaultParent)
  const [parentTouched, setParentTouched] = useState(false)
  const [sessionCandidate, setSessionCandidate] = useState<string | null>(null)
  const [stagedFrom, setStagedFrom] = useState<string | null>(null)
  const [browserOpen, setBrowserOpen] = useState(false)
  const [editedFolderName, setEditedFolderName] = useState('')
  const [folderNameEdited, setFolderNameEdited] = useState(false)
  const [root, setRoot] = useState('')

  const playlists = usePlaylists()
  const channels = useChannels()

  // The videos root is read once from a listing of the root itself, so the
  // destination has its absolute prefix even when no folder is ever browsed.
  useEffect(() => {
    let cancelled = false
    fetchDirectories('')
      .then((rootListing) => {
        if (!cancelled) {
          setRoot(rootListing.root)
        }
      })
      .catch(() => {})
    return () => {
      cancelled = true
    }
  }, [])

  const occupied = useMemo(() => {
    const taken = new Map<string, string>()
    playlists.data?.forEach((playlist) => taken.set(playlist.path, playlist.name))
    channels.data?.forEach((channel) => taken.set(channel.path, channel.name))
    return taken
  }, [playlists.data, channels.data])

  // Suggestions derive from the dialog's own kind only: the other kind's
  // folders are where the user files the other kind.
  const ownItems = useMemo(
    () => (mode === 'playlist' ? playlists.data : channels.data) ?? [],
    [mode, playlists.data, channels.data],
  )

  const candidates = useMemo(() => {
    const derived = deriveSaveCandidates(defaultParent, ownItems)
    if (sessionCandidate && !derived.some((candidate) => candidate.path === sessionCandidate)) {
      derived.push({
        path: sessionCandidate,
        count: ownItems.filter((item) => parentOf(item.path) === sessionCandidate).length,
      })
    }
    return derived
  }, [defaultParent, ownItems, sessionCandidate])

  // The remembered parent only preselects while it is still among the
  // derived candidates: a stale one falls back to the default silently. It
  // never overrides a selection the user already made this session.
  useEffect(() => {
    if (parentTouched || !ownItems.length) {
      return
    }
    const remembered = readRememberedParent(mode)
    if (
      remembered &&
      remembered !== defaultParent &&
      deriveSaveCandidates(defaultParent, ownItems).some((c) => c.path === remembered)
    ) {
      setParent(remembered)
    }
  }, [mode, defaultParent, ownItems, parentTouched])

  const selectParent = useCallback((next: string) => {
    setParentTouched(true)
    setParent(next)
    setStagedFrom((prev) => (isStaged(next, prev) ? prev : null))
  }, [])

  const stage = useCallback((next: string) => {
    setParentTouched(true)
    setParent(next)
    setStagedFrom((prev) => prev ?? next)
  }, [])

  const closeBrowser = useCallback(() => {
    setBrowserOpen(false)
    // The browsed folder stays selectable in the list for the rest of the
    // dialog session, even against other candidates.
    setSessionCandidate(parent)
  }, [parent])

  // The folder name auto-fills from the source field until it is edited by
  // hand, and from then on is whatever was typed, for the rest of the dialog
  // session.
  const folderName = folderNameEdited ? editedFolderName : slugify(nameSource)
  const setFolderName = useCallback((name: string) => {
    setEditedFolderName(name)
    setFolderNameEdited(true)
  }, [])

  const folderNameError = useMemo(() => {
    if (!folderName) {
      return 'Folder name is required.'
    }
    if (folderName.includes('/')) {
      return 'Folder name must not contain "/". Choose the parent folder from the list instead.'
    }
    return null
  }, [folderName])

  const destinationPath = folderName ? join(parent, folderName) : ''
  const occupiedBy = destinationPath ? occupied.get(destinationPath) : undefined
  const valid = !folderNameError && !occupiedBy
  // Absolute only once the videos root is known, so no caller ever shows a
  // path that reads as absolute but is missing its prefix.
  const destination = root && !folderNameError ? `${root}/${destinationPath}` : ''

  return {
    parent,
    candidates,
    selectParent,
    browserOpen,
    openBrowser: () => setBrowserOpen(true),
    closeBrowser,
    folderName,
    setFolderName,
    folderNameError,
    occupied,
    root,
    stagedFrom,
    stage,
    value: { path: destinationPath, destination, valid, occupiedBy },
    remember: () => writeRememberedParent(mode, parent),
  }
}
