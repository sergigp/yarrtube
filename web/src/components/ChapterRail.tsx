import { useEffect, useRef } from 'react'
import type { Chapter } from '@/api/types'
import { currentChapterIndex } from '@/lib/chapters'
import { formatDuration } from '@/lib/formatDuration'
import { cn } from '@/lib/utils'

interface ChapterRailProps {
  chapters: Chapter[]
  currentTime: number
  onPick: (chapter: Chapter) => void
}

/**
 * The chapters of the playing video as a row of same-width cards under the
 * player, the current one highlighted and kept in view.
 */
export function ChapterRail({ chapters, currentTime, onPick }: ChapterRailProps) {
  const current = currentChapterIndex(chapters, currentTime)
  const listRef = useRef<HTMLUListElement>(null)

  // Centre the current card in the rail, only when the current chapter
  // changes, so a user scrolling the rail isn't pulled back on every tick.
  // Scrolls the rail itself, never the page.
  useEffect(() => {
    const list = listRef.current
    const card = current == null ? null : list?.children[current]
    if (!list || !(card instanceof HTMLElement)) {
      return
    }
    const left = card.offsetLeft - (list.clientWidth - card.offsetWidth) / 2
    list.scrollTo({ left, behavior: 'smooth' })
  }, [current])

  return (
    <nav aria-label="Chapters" className="px-4 py-2 sm:px-6 md:p-0">
      <ul ref={listRef} className="relative flex gap-2 overflow-x-auto">
        {chapters.map((chapter, index) => (
          <li key={chapter.start_seconds} className="shrink-0">
            <button
              type="button"
              title={chapter.title}
              onClick={() => onPick(chapter)}
              aria-current={index === current ? 'true' : undefined}
              className={cn(
                'flex w-40 flex-col items-start gap-0.5 rounded-md border border-border px-2.5 py-1.5 text-left transition-colors hover:bg-accent',
                index === current && 'border-primary bg-accent',
              )}
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
