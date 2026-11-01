import type { Chapter } from '@/api/types'

/** Index of the chapter containing `time` (start inclusive, end exclusive), or null. */
export function currentChapterIndex(_chapters: Chapter[], _time: number): number | null {
  return null
}

/** 0..1 progress of `time` through `chapter`, clamped. */
export function chapterProgress(_chapter: Chapter, _time: number): number {
  return 0
}
