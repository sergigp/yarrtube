import type { Chapter } from '@/api/types'
import { useCurrentTime } from '@/hooks/useCurrentTime'
import { currentChapterIndex } from '@/lib/chapters'
import { formatDuration } from '@/lib/formatDuration'
import { playFrom } from '@/lib/playFrom'
import { cn } from '@/lib/utils'

/**
 * The playing video's chapters as a compact list beside its AI summary
 * (desktop): start time and title per row, the current one highlighted.
 * Picking one plays the video from its start.
 */
export function ChapterList({
  chapters,
  videoElement,
}: {
  chapters: Chapter[]
  videoElement: HTMLVideoElement | null
}) {
  const currentTime = useCurrentTime(videoElement)
  const current = currentChapterIndex(chapters, currentTime)
  return (
    <nav
      aria-label="Chapters"
      className="flex min-w-0 flex-[1_1_15rem] flex-col gap-0.5 rounded-xl bg-secondary/70 px-2 py-3 dark:bg-card"
    >
      <h4 className="mb-1.5 ml-2 text-xs font-semibold tracking-wider text-muted-foreground uppercase">
        Chapters
      </h4>
      <ul className="flex max-h-72 flex-col gap-0.5 overflow-y-auto">
        {chapters.map((chapter, index) => (
          <li key={chapter.start_seconds}>
            <button
              type="button"
              title={chapter.title}
              aria-current={index === current ? 'true' : undefined}
              onClick={() => videoElement && playFrom(videoElement, chapter.start_seconds)}
              className={cn(
                'flex h-8 w-full items-center gap-3 rounded-lg px-2 text-left text-sm transition-colors hover:bg-background dark:hover:bg-accent',
                index === current && 'bg-background font-medium dark:bg-accent',
              )}
            >
              <span className="w-12 shrink-0 text-xs text-muted-foreground tabular-nums">
                {formatDuration(chapter.start_seconds)}
              </span>
              <span className="min-w-0 flex-1 truncate">{chapter.title}</span>
            </button>
          </li>
        ))}
      </ul>
    </nav>
  )
}
