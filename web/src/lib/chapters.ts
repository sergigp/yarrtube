import type { Chapter } from '@/api/types'

/** Index of the chapter containing `time` (start inclusive, end exclusive), or null. */
export function currentChapterIndex(chapters: Chapter[], time: number): number | null {
  const index = chapters.findIndex(
    (chapter) => time >= chapter.start_seconds && time < chapter.end_seconds,
  )
  return index === -1 ? null : index
}

/** 0..1 progress of `time` through `chapter`, clamped. */
export function chapterProgress(_chapter: Chapter, _time: number): number {
  return 0
}
