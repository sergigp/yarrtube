import { Folder, FolderSearch } from 'lucide-react'
import type { SaveLocation } from '@/hooks/useSaveLocation'
import { FolderBrowser } from './FolderBrowser'

/**
 * The always-visible "Save to" single-choice list of suggested parent
 * folders, with the folder browser inline behind "Choose another folder…".
 */
export function SaveToField({ location }: { location: SaveLocation }) {
  return (
    <div className="flex min-w-0 flex-col gap-2">
      <p className="text-sm leading-none font-medium">Save to</p>
      {location.browserOpen ? (
        <FolderBrowser
          parent={location.parent}
          root={location.root}
          occupied={location.occupied}
          stagedFrom={location.stagedFrom}
          onNavigate={location.selectParent}
          onStage={location.stage}
          onClose={location.closeBrowser}
        />
      ) : (
        <div
          role="radiogroup"
          aria-label="Save to"
          className="flex min-w-0 flex-col divide-y divide-border overflow-hidden rounded-md border border-input"
        >
          {location.candidates.map((candidate) => {
            const selected = candidate.path === location.parent
            return (
              <label
                key={candidate.path}
                className={`flex min-w-0 cursor-pointer items-center gap-2.5 px-3 py-2.5 transition-colors hover:bg-muted/60 ${
                  selected ? 'bg-muted/40' : ''
                }`}
              >
                <input
                  type="radio"
                  name="save-to"
                  aria-label={`${candidate.path}/`}
                  checked={selected}
                  onChange={() => location.selectParent(candidate.path)}
                  className="accent-primary"
                />
                <Folder className="size-3.5 shrink-0 text-muted-foreground" />
                <span className="truncate font-mono text-xs">{`${candidate.path}/`}</span>
                {candidate.count > 0 && (
                  <span className="ml-auto shrink-0 pl-2 text-xs text-muted-foreground">
                    {`· ${candidate.count} ${candidate.count === 1 ? 'item' : 'items'}`}
                  </span>
                )}
              </label>
            )
          })}
          <button
            type="button"
            onClick={location.openBrowser}
            className="flex min-w-0 items-center gap-2.5 px-3 py-2.5 text-left text-xs text-muted-foreground transition-colors hover:bg-muted/60 hover:text-foreground"
          >
            <FolderSearch className="size-3.5 shrink-0" />
            Choose another folder…
          </button>
        </div>
      )}
    </div>
  )
}
