import { useEffect, useState } from 'react'
import { Check, ChevronRight, Folder, FolderPlus } from 'lucide-react'
import { fetchDirectories } from '@/api/client'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'

interface FolderBrowserProps {
  parent: string
  root: string
  occupied: Map<string, string>
  stagedFrom: string | null
  onNavigate: (path: string) => void
  onStage: (path: string) => void
  onClose: () => void
}

function join(parent: string, name: string): string {
  return parent ? `${parent}/${name}` : name
}

/**
 * Whether `path` is the staged parent or sits below it. A staged parent does
 * not exist on disk, so neither it nor anything under it can be listed.
 */
function isStaged(path: string, stagedFrom: string | null): boolean {
  return Boolean(stagedFrom) && (path === stagedFrom || path.startsWith(`${stagedFrom}/`))
}

function breadcrumb(parent: string, rootLabel: string): { label: string; path: string }[] {
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
 * the root; the full absolute path belongs in the dialog's destination notice.
 */
function shortRootLabel(root: string): string {
  return root.split('/').filter(Boolean).pop() ?? 'videos'
}

/**
 * The breadcrumb + subdirectory browser behind "Choose another folder…".
 * Browsing live-updates the selected parent through `onNavigate`, so the
 * dialog's destination follows every step; `onClose` returns to the
 * "Save to" list with the browsed folder as its selected candidate.
 */
export function FolderBrowser({
  parent,
  root,
  occupied,
  stagedFrom,
  onNavigate,
  onStage,
  onClose,
}: FolderBrowserProps) {
  const [listing, setListing] = useState<{ path: string | null; entries: string[] }>({
    path: null,
    entries: [],
  })
  const [listingError, setListingError] = useState<Error | null>(null)
  const [newFolderOpen, setNewFolderOpen] = useState(false)
  const [newFolderName, setNewFolderName] = useState('')
  const [newFolderError, setNewFolderError] = useState<string | null>(null)

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
          setListingError(null)
        }
      })
      .catch((err: Error) => {
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

  const goTo = (next: string) => {
    onNavigate(next)
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
      onStage(next)
      setNewFolderOpen(false)
      setNewFolderError(null)
    }
    setNewFolderName('')
  }

  const crumbs = breadcrumb(parent, shortRootLabel(root))

  return (
    <div className="flex min-w-0 flex-col gap-2 rounded-md border border-border p-2.5">
      <nav aria-label="Folder path" className="flex min-w-0 flex-wrap items-center gap-0.5 text-xs">
        {crumbs.map((crumb, index) => {
          const isCurrent = index === crumbs.length - 1
          return (
            <span key={crumb.path} className="flex min-w-0 items-center gap-0.5">
              {index > 0 && <ChevronRight className="size-3 shrink-0 text-muted-foreground" />}
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
          {staged ? 'Does not exist yet — created on the first download.' : 'No subfolders here.'}
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
                    <span className="ml-auto min-w-0 shrink truncate pl-2 text-[11px] text-muted-foreground">
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

      <button
        type="button"
        onClick={onClose}
        className="flex items-center gap-1.5 self-start rounded-md border border-input px-2 py-1 text-xs hover:bg-muted"
      >
        <Check className="size-3.5" />
        Use this folder
      </button>
    </div>
  )
}
