import { useEffect, useMemo, useState } from 'react'
import { ChevronRight, Folder, FolderPlus } from 'lucide-react'
import { fetchChannels, fetchDirectories, fetchPlaylists } from '../api'
import { slugify } from '../slugify'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Button } from '@/components/ui/button'

const DEFAULT_PARENTS = { playlist: 'playlists', channel: 'channels' }

function join(parent, name) {
  return parent ? `${parent}/${name}` : name
}

/**
 * Whether `path` is the staged parent or sits below it. A staged parent does
 * not exist on disk, so neither it nor anything under it can be listed.
 */
function isStaged(path, stagedFrom) {
  return Boolean(stagedFrom) && (path === stagedFrom || path.startsWith(`${stagedFrom}/`))
}

/**
 * The segments of `parent` that do not exist yet: everything from the staged
 * ancestor downwards. A parent adopted through the create-folder step can be
 * several levels deep, so this is a list, not a single name.
 */
function stagedSegments(parent, stagedFrom) {
  if (!isStaged(parent, stagedFrom)) {
    return []
  }
  return parent.split('/').slice(stagedFrom.split('/').length - 1)
}

function breadcrumb(parent, rootLabel) {
  const segments = parent ? parent.split('/') : []
  return [
    { label: rootLabel, path: '' },
    ...segments.map((segment, index) => ({
      label: segment,
      path: segments.slice(0, index + 1).join('/'),
    })),
  ]
}

/**
 * The storage location controls: a parent folder chosen only by browsing what
 * is on disk, and a folder name for the directory the videos land in.
 *
 * `onChange` must be referentially stable (wrap it in `useCallback`): it is
 * called from an effect whenever the composed destination or its validity
 * changes.
 */
export function LocationField({ mode, nameSource, onChange }) {
  const [parent, setParent] = useState(DEFAULT_PARENTS[mode])
  const [stagedFrom, setStagedFrom] = useState(null)
  const [browserOpen, setBrowserOpen] = useState(false)
  const [listing, setListing] = useState({ path: null, entries: [] })
  const [listingError, setListingError] = useState(null)
  const [root, setRoot] = useState('')
  const [occupied, setOccupied] = useState(new Map())
  const [editedFolderName, setEditedFolderName] = useState('')
  const [folderNameEdited, setFolderNameEdited] = useState(false)
  const [newFolderOpen, setNewFolderOpen] = useState(false)
  const [newFolderName, setNewFolderName] = useState('')
  const [newFolderError, setNewFolderError] = useState(null)

  // The videos root is read once from a listing of the root itself, so the
  // destination preview has its absolute prefix even when the current
  // parent's own listing fails or is staged.
  useEffect(() => {
    let cancelled = false
    fetchDirectories('')
      .then((listing) => {
        if (!cancelled) {
          setRoot(listing.root)
        }
      })
      .catch(() => {})
    return () => {
      cancelled = true
    }
  }, [])

  useEffect(() => {
    let cancelled = false
    Promise.all([fetchPlaylists(), fetchChannels()])
      .then(([playlists, channels]) => {
        if (cancelled) {
          return
        }
        const taken = new Map()
        playlists.forEach((playlist) => taken.set(playlist.path, playlist.name))
        channels.forEach((channel) => taken.set(channel.path, channel.name))
        setOccupied(taken)
      })
      .catch(() => {})
    return () => {
      cancelled = true
    }
  }, [])

  // A staged parent is never requested: it does not exist on disk, and asking
  // for it would only produce a not-found error.
  const staged = isStaged(parent, stagedFrom)
  useEffect(() => {
    if (staged) {
      return undefined
    }

    let cancelled = false
    fetchDirectories(parent)
      .then((next) => {
        if (!cancelled) {
          setListing({ path: parent, entries: next.entries.map((entry) => entry.name) })
          setRoot(next.root)
          setListingError(null)
        }
      })
      .catch((err) => {
        if (!cancelled) {
          setListing({ path: parent, entries: [] })
          setListingError(err)
        }
      })
    return () => {
      cancelled = true
    }
  }, [parent, staged])

  // Derived rather than stored, so a listing still in flight (or one for the
  // parent we just navigated away from) never shows as this parent's contents.
  const entries = staged || listing.path !== parent ? [] : listing.entries
  // The folder name auto-fills from the source field until it is edited by
  // hand, and from then on is whatever was typed, for the rest of the dialog
  // session.
  const folderName = folderNameEdited ? editedFolderName : slugify(nameSource)

  const newParentSegments = stagedSegments(parent, stagedFrom)
  const destinationPath = folderName ? join(parent, folderName) : ''
  const occupiedBy = destinationPath ? occupied.get(destinationPath) : undefined
  const leafExists = !staged && entries.includes(folderName)
  const newDirectories = [...newParentSegments, ...(folderName && !leafExists ? [folderName] : [])]

  const folderNameError = useMemo(() => {
    if (!folderName) {
      return 'Folder name is required.'
    }
    if (folderName.includes('/')) {
      return 'Folder name must not contain "/". Choose the parent folder by browsing instead.'
    }
    return null
  }, [folderName])

  const valid = !folderNameError && !occupiedBy
  useEffect(() => {
    onChange({ path: destinationPath, valid })
  }, [onChange, destinationPath, valid])

  const goTo = (next) => {
    setParent(next)
    if (!isStaged(next, stagedFrom)) {
      setStagedFrom(null)
    }
    setNewFolderOpen(false)
    setNewFolderError(null)
  }

  const adoptFolder = () => {
    const name = newFolderName.trim()
    if (!name) {
      setNewFolderError('Folder name is required.')
      return
    }
    if (name.includes('/')) {
      setNewFolderError('Folder name must not contain "/".')
      return
    }

    const next = join(parent, name)
    if (entries.includes(name)) {
      // Naming something that is already there is a way of reaching it, not a
      // request to create a second one.
      goTo(next)
    } else {
      setParent(next)
      setStagedFrom((prev) => prev ?? next)
      setNewFolderOpen(false)
      setNewFolderError(null)
    }
    setNewFolderName('')
  }

  const rootLabel = root || 'videos'
  const crumbs = breadcrumb(parent, rootLabel)

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-1.5">
        <Label htmlFor="location-parent">Parent folder</Label>
        <div className="flex items-center gap-2">
          <span
            id="location-parent"
            className="min-w-0 flex-1 truncate rounded-md border border-input bg-muted/40 px-3 py-2 font-mono text-sm"
          >
            {parent ? `${parent}/` : `${rootLabel}/`}
          </span>
          <Button
            type="button"
            variant="outline"
            onClick={() => setBrowserOpen((prev) => !prev)}
            aria-expanded={browserOpen}
          >
            {browserOpen ? 'Done' : 'Change'}
          </Button>
        </div>
      </div>

      {browserOpen && (
        <div className="flex flex-col gap-3 rounded-md border border-border p-3">
          <nav aria-label="Folder path" className="flex flex-wrap items-center gap-1 text-sm">
            {crumbs.map((crumb, index) => {
              const isCurrent = index === crumbs.length - 1
              return (
                <span key={crumb.path} className="flex items-center gap-1">
                  {index > 0 && <ChevronRight className="size-3.5 text-muted-foreground" />}
                  {isCurrent ? (
                    <span className="font-mono font-medium">{crumb.label}</span>
                  ) : (
                    <button
                      type="button"
                      className="font-mono text-muted-foreground underline-offset-2 hover:text-foreground hover:underline"
                      onClick={() => goTo(crumb.path)}
                    >
                      {crumb.label}
                    </button>
                  )}
                </span>
              )
            })}
          </nav>

          {!staged && listingError ? (
            <p className="text-sm text-destructive">{listingError.message}</p>
          ) : entries.length === 0 ? (
            <p className="text-sm text-muted-foreground">
              {staged
                ? 'This folder does not exist yet and will be created on the first download.'
                : 'This folder has no subfolders.'}
            </p>
          ) : (
            <ul className="flex max-h-48 flex-col gap-0.5 overflow-y-auto">
              {entries.map((name) => {
                const takenBy = occupied.get(join(parent, name))
                return (
                  <li key={name}>
                    <button
                      type="button"
                      className="flex w-full items-center gap-2 rounded px-2 py-1.5 text-left text-sm hover:bg-muted"
                      onClick={() => goTo(join(parent, name))}
                    >
                      <Folder className="size-4 shrink-0 text-muted-foreground" />
                      <span className="truncate font-mono">{name}</span>
                      {takenBy && (
                        <span className="ml-auto shrink-0 text-xs text-muted-foreground">
                          in use by {takenBy}
                        </span>
                      )}
                    </button>
                  </li>
                )
              })}
            </ul>
          )}

          {newFolderOpen ? (
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="location-new-folder">New folder name</Label>
              <div className="flex items-center gap-2">
                <Input
                  id="location-new-folder"
                  type="text"
                  value={newFolderName}
                  onChange={(event) => {
                    setNewFolderName(event.target.value)
                    setNewFolderError(null)
                  }}
                  onKeyDown={(event) => {
                    if (event.key === 'Enter') {
                      event.preventDefault()
                      adoptFolder()
                    }
                  }}
                />
                <Button type="button" onClick={adoptFolder}>
                  Use folder
                </Button>
              </div>
              {newFolderError && <p className="text-sm text-destructive">{newFolderError}</p>}
            </div>
          ) : (
            <button
              type="button"
              className="flex items-center gap-2 self-start text-sm text-muted-foreground hover:text-foreground"
              onClick={() => setNewFolderOpen(true)}
            >
              <FolderPlus className="size-4" />
              New folder
            </button>
          )}
        </div>
      )}

      <div className="flex flex-col gap-1.5">
        <Label htmlFor="location-folder-name">Folder name</Label>
        <Input
          id="location-folder-name"
          type="text"
          value={folderName}
          onChange={(event) => {
            setEditedFolderName(event.target.value)
            setFolderNameEdited(true)
          }}
          aria-invalid={Boolean(folderNameError)}
        />
        {folderNameError && <p className="text-sm text-destructive">{folderNameError}</p>}
      </div>

      <div className="flex flex-col gap-1 rounded-md bg-muted/40 p-3">
        <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
          Download destination
        </p>
        <p className="break-all font-mono text-sm" data-testid="destination-path">
          {root ? `${root}/` : ''}
          {folderNameError ? (
            <>
              {parent && `${parent}/`}
              <span className="text-muted-foreground">…</span>
            </>
          ) : (
            destinationPath
          )}
        </p>
        {/* An invalid folder name composes no destination worth describing,
            and the field already says why — claiming it "will be created"
            alongside that error would contradict it. */}
        {folderNameError ? null : occupiedBy ? (
          <p className="text-sm text-destructive">
            Already used by {occupiedBy}. Choose a different folder.
          </p>
        ) : newDirectories.length > 0 ? (
          <p className="text-sm text-muted-foreground">
            Will create {newDirectories.length === 1 ? 'a new folder' : 'new folders'}:{' '}
            {newDirectories.join(', ')}
          </p>
        ) : destinationPath ? (
          <p className="text-sm text-muted-foreground">
            This folder already exists. Videos will be added to its contents.
          </p>
        ) : null}
      </div>
    </div>
  )
}
