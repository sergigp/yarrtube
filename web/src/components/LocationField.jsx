import { useEffect, useMemo, useState } from 'react'
import { ChevronRight, Folder, FolderPlus } from 'lucide-react'
import { fetchChannels, fetchDirectories, fetchPlaylists } from '../api'
import { slugify } from '../slugify'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'

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
 * The videos root's own directory name. The breadcrumb only needs to identify
 * the root, and spelling out a deep absolute path there is what used to force
 * the dialog wider than its own width; the full path belongs in the
 * destination, which is the one place it is the point.
 */
function shortRootLabel(root) {
  return root.split('/').filter(Boolean).pop() ?? 'videos'
}

/**
 * The storage location: a parent folder chosen only by browsing what is on
 * disk, and a folder name for the directory the videos land in. The two read
 * as one path-shaped control, with the resolved destination below it carrying
 * the visual weight — the destination is the outcome the user is deciding,
 * the controls are only how it gets composed.
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
  // destination has its absolute prefix even when the current parent's own
  // listing fails or is staged.
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

  const rootLabel = shortRootLabel(root)
  const crumbs = breadcrumb(parent, rootLabel)
  const parentDisplay = parent ? `${parent}/` : `${rootLabel}/`

  return (
    <div className="flex flex-col gap-3">
      <div className="flex min-w-0 flex-col gap-1.5">
        <p className="text-sm leading-none font-medium">Location</p>
        {/* Parent and folder name read as one path. A bare input rather than
            the `Input` component, so the two sit seamlessly in one frame. */}
        <div className="flex min-w-0 items-stretch rounded-md border border-input focus-within:border-ring focus-within:ring-3 focus-within:ring-ring/50">
          <button
            type="button"
            onClick={() => setBrowserOpen((prev) => !prev)}
            aria-expanded={browserOpen}
            aria-label="Parent folder"
            title={root ? `${root}/${parent}` : parentDisplay}
            className="flex min-w-0 shrink items-center gap-1.5 rounded-l-md border-r border-input bg-muted/50 px-2.5 py-2 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
          >
            <Folder className="size-3.5 shrink-0" />
            <span className="truncate font-mono text-xs">{parentDisplay}</span>
          </button>
          <input
            aria-label="Folder name"
            type="text"
            // `min-width: 0` governs how this shrinks once laid out, but the
            // default 20-character intrinsic size still feeds the grid's
            // min-content pass, which widened the whole dialog on narrow
            // screens. `flex-1` supplies the real width.
            size={1}
            value={folderName}
            onChange={(event) => {
              setEditedFolderName(event.target.value)
              setFolderNameEdited(true)
            }}
            aria-invalid={Boolean(folderNameError)}
            className="min-w-0 flex-1 rounded-r-md bg-transparent px-2.5 py-2 font-mono text-sm outline-none"
          />
        </div>
        {folderNameError && <p className="text-sm text-destructive">{folderNameError}</p>}
      </div>

      {browserOpen && (
        <div className="flex min-w-0 flex-col gap-2 rounded-md border border-border p-2">
          <nav
            aria-label="Folder path"
            className="flex min-w-0 flex-wrap items-center gap-0.5 text-xs"
          >
            {crumbs.map((crumb, index) => {
              const isCurrent = index === crumbs.length - 1
              return (
                <span key={crumb.path} className="flex min-w-0 items-center gap-0.5">
                  {index > 0 && (
                    <ChevronRight className="size-3 shrink-0 text-muted-foreground" />
                  )}
                  {isCurrent ? (
                    <span className="truncate font-mono font-medium">{crumb.label}</span>
                  ) : (
                    <button
                      type="button"
                      className="truncate font-mono text-muted-foreground underline-offset-2 hover:text-foreground hover:underline"
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
            <p className="text-xs text-destructive">{listingError.message}</p>
          ) : entries.length === 0 ? (
            <p className="px-1 py-0.5 text-xs text-muted-foreground">
              {staged
                ? 'Does not exist yet — created on the first download.'
                : 'No subfolders here.'}
            </p>
          ) : (
            <ul className="flex max-h-40 min-w-0 flex-col overflow-y-auto">
              {entries.map((name) => {
                const takenBy = occupied.get(join(parent, name))
                return (
                  <li key={name} className="min-w-0">
                    <button
                      type="button"
                      className="flex w-full min-w-0 items-center gap-1.5 rounded px-1.5 py-1 text-left text-xs hover:bg-muted"
                      onClick={() => goTo(join(parent, name))}
                    >
                      <Folder className="size-3.5 shrink-0 text-muted-foreground" />
                      <span className="truncate font-mono">{name}</span>
                      {takenBy && (
                        <span className="ml-auto shrink-0 truncate pl-2 text-[11px] text-muted-foreground">
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
            <div className="flex min-w-0 flex-col gap-1.5">
              <Label htmlFor="location-new-folder" className="text-xs">
                New folder name
              </Label>
              <div className="flex min-w-0 items-center gap-1.5">
                <Input
                  id="location-new-folder"
                  type="text"
                  className="h-8 min-w-0 flex-1 text-sm"
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
                <button
                  type="button"
                  onClick={adoptFolder}
                  className="shrink-0 rounded-md border border-input px-2 py-1 text-xs hover:bg-muted"
                >
                  Use folder
                </button>
              </div>
              {newFolderError && <p className="text-xs text-destructive">{newFolderError}</p>}
            </div>
          ) : (
            <button
              type="button"
              className="flex items-center gap-1.5 self-start px-1.5 text-xs text-muted-foreground hover:text-foreground"
              onClick={() => setNewFolderOpen(true)}
            >
              <FolderPlus className="size-3.5" />
              New folder
            </button>
          )}
        </div>
      )}

      {/* The result of every control above it, so it carries the weight: the
          user is choosing where videos end up, not which widget to click. */}
      <div className="min-w-0 rounded-md border-l-2 border-primary bg-muted/40 px-3 py-2">
        <p className="text-[10px] font-semibold tracking-wider text-muted-foreground uppercase">
          Download destination
        </p>
        <p
          className="mt-1 wrap-anywhere font-mono text-sm font-medium text-foreground"
          data-testid="destination-path"
        >
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
        {folderNameError ? null : occupiedBy ? (
          <p className="mt-1 text-xs font-medium text-destructive">
            Already used by {occupiedBy}. Choose a different folder.
          </p>
        ) : newDirectories.length > 0 ? (
          <p className="mt-1 text-xs text-muted-foreground">
            Will create {newDirectories.length === 1 ? 'a new folder' : 'new folders'}:{' '}
            {newDirectories.join(', ')}
          </p>
        ) : destinationPath ? (
          <p className="mt-1 text-xs text-muted-foreground">
            This folder already exists. Videos will be added to its contents.
          </p>
        ) : null}
      </div>
    </div>
  )
}
