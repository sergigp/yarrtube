import type { Chapter } from '@/api/types'
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog'
import { formatDuration } from '@/lib/formatDuration'
import { cn } from '@/lib/utils'

interface ChapterListSheetProps {
  chapters: Chapter[]
  currentIndex: number | null
  open: boolean
  onOpenChange: (open: boolean) => void
  onPick: (chapter: Chapter) => void
}

/**
 * Every chapter of the playing video in a sheet rising from the bottom of
 * the screen, one per row with its full title, the current one marked.
 * Picking a chapter closes it.
 */
export function ChapterListSheet({
  chapters,
  currentIndex,
  open,
  onOpenChange,
  onPick,
}: ChapterListSheetProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="top-auto bottom-0 left-0 flex max-h-[70vh] max-w-none translate-x-0 translate-y-0 flex-col gap-3 rounded-b-none border-x-0 border-b-0 p-4 sm:max-w-none data-open:zoom-in-100 data-open:slide-in-from-bottom-8 data-closed:zoom-out-100">
        <DialogTitle>Chapters</DialogTitle>
        <DialogDescription className="sr-only">
          Pick a chapter to play the video from its start.
        </DialogDescription>
        <ul className="-mx-2 flex min-h-0 flex-col overflow-y-auto">
          {chapters.map((chapter, index) => (
            <li key={chapter.start_seconds}>
              <button
                type="button"
                aria-current={index === currentIndex ? 'true' : undefined}
                onClick={() => {
                  onPick(chapter)
                  onOpenChange(false)
                }}
                className={cn(
                  'flex w-full items-baseline gap-3 rounded-md px-2 py-2.5 text-left transition-colors hover:bg-accent',
                  index === currentIndex && 'bg-accent font-medium',
                )}
              >
                <span className="w-14 shrink-0 text-xs text-muted-foreground tabular-nums">
                  {formatDuration(chapter.start_seconds)}
                </span>{' '}
                <span className="text-sm text-foreground">{chapter.title}</span>
              </button>
            </li>
          ))}
        </ul>
      </DialogContent>
    </Dialog>
  )
}
