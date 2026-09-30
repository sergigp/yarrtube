import type { SaveLocation } from '@/hooks/useSaveLocation'
import { FolderBrowser } from './FolderBrowser'

/**
 * The always-visible "Save to" single-choice list of suggested parent
 * folders, with the folder browser inline behind "Choose another folder…".
 */
export function SaveToField({ location }: { location: SaveLocation }) {
  return (
    <div className="flex min-w-0 flex-col gap-1.5">
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
        <div role="radiogroup" aria-label="Save to" className="flex min-w-0 flex-col">
          {location.candidates.map((candidate) => (
            <label key={candidate.path} className="flex min-w-0 items-center gap-2">
              <input
                type="radio"
                name="save-to"
                aria-label={`${candidate.path}/`}
                checked={candidate.path === location.parent}
                onChange={() => location.selectParent(candidate.path)}
              />
              <span className="truncate font-mono text-xs">{`${candidate.path}/`}</span>
              {candidate.count > 0 && (
                <span className="ml-auto shrink-0 text-xs text-muted-foreground">
                  {`· ${candidate.count} ${candidate.count === 1 ? 'item' : 'items'}`}
                </span>
              )}
            </label>
          ))}
          <button type="button" onClick={location.openBrowser} className="self-start text-xs">
            Choose another folder…
          </button>
        </div>
      )}
    </div>
  )
}
