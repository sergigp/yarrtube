import type { Chapter } from '@/api/types'

interface ChapterRailProps {
  chapters: Chapter[]
  currentTime: number
  onPick: (chapter: Chapter) => void
}

/**
 * The chapters of the playing video as a row of same-width cards under the
 * player, the current one highlighted and kept in view.
 */
export function ChapterRail(_props: ChapterRailProps) {
  return <div />
}
