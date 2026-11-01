import type { Chapter } from '@/api/types'

/** Index of the chapter containing `time` (start inclusive, end exclusive), or null. */
export function currentChapterIndex(chapters: Chapter[], time: number): number | null {
  const index = chapters.findIndex(
    (chapter) => time >= chapter.start_seconds && time < chapter.end_seconds,
  )
  return index === -1 ? null : index
}

/** 0..1 progress of `time` through `chapter`, clamped. */
export function chapterProgress(chapter: Chapter, time: number): number {
  const length = chapter.end_seconds - chapter.start_seconds
  if (length <= 0) {
    return 0
  }
  return Math.min(Math.max((time - chapter.start_seconds) / length, 0), 1)
}
