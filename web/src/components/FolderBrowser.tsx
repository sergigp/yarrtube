interface FolderBrowserProps {
  parent: string
  root: string
  occupied: Map<string, string>
  stagedFrom: string | null
  onNavigate: (path: string) => void
  onStage: (path: string) => void
  onClose: () => void
}

/**
 * The breadcrumb + subdirectory browser behind "Choose another folder…".
 * Navigation live-updates the selected parent through `onNavigate`.
 */
export function FolderBrowser({ parent, onClose }: FolderBrowserProps) {
  return (
    <div className="flex min-w-0 flex-col gap-2 rounded-md border border-border p-2">
      <span className="truncate font-mono text-xs">{parent}</span>
      <button type="button" onClick={onClose} className="self-start text-xs">
        Done
      </button>
    </div>
  )
}

export type { FolderBrowserProps }
