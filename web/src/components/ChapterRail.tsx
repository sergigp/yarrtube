import type { Chapter } from '@/api/types'
import { formatDuration } from '@/lib/formatDuration'

interface ChapterRailProps {
  chapters: Chapter[]
  currentTime: number
  onPick: (chapter: Chapter) => void
}

/**
 * The chapters of the playing video as a row of same-width cards under the
 * player, the current one highlighted and kept in view.
 */
export function ChapterRail({ chapters }: ChapterRailProps) {
  return (
    <nav aria-label="Chapters" className="px-4 py-2 sm:px-6 md:p-0">
      <ul className="flex gap-2 overflow-x-auto">
        {chapters.map((chapter) => (
          <li key={chapter.start_seconds} className="shrink-0">
            <button
              type="button"
              title={chapter.title}
              className="flex w-40 flex-col items-start gap-0.5 rounded-md border border-border px-2.5 py-1.5 text-left transition-colors hover:bg-accent"
            >
              <span className="text-xs text-muted-foreground tabular-nums">
                {formatDuration(chapter.start_seconds)}
              </span>{' '}
              <span className="line-clamp-2 text-sm leading-snug text-foreground">
                {chapter.title}
              </span>
            </button>
          </li>
        ))}
      </ul>
    </nav>
  )
}
