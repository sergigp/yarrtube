import type { Chapter } from '@/api/types'

interface ChapterListSheetProps {
  chapters: Chapter[]
  currentIndex: number | null
  open: boolean
  onOpenChange: (open: boolean) => void
  onPick: (chapter: Chapter) => void
}

/** Every chapter of the playing video in a sheet, one per row (mobile). */
export function ChapterListSheet(_props: ChapterListSheetProps) {
  return null
}
