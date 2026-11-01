import { describe, expect, it } from 'vitest'
import type { Chapter } from '@/api/types'
import { chapterProgress, currentChapterIndex } from './chapters'

const CHAPTERS: Chapter[] = [
  { start_seconds: 0, end_seconds: 75, title: 'Intro' },
  { start_seconds: 75, end_seconds: 1037, title: 'Game 1' },
]

describe('currentChapterIndex', () => {
  it('finds the chapter containing the time', () => {
    expect(currentChapterIndex(CHAPTERS, 500)).toBe(1)
  })

  it('counts a chapter start as inside it', () => {
    expect(currentChapterIndex(CHAPTERS, 75)).toBe(1)
  })

  it('counts a chapter end as outside it', () => {
    expect(currentChapterIndex(CHAPTERS, 74.9)).toBe(0)
    expect(currentChapterIndex(CHAPTERS, 1037)).toBeNull()
  })

  it('finds none after the last chapter or without chapters', () => {
    expect(currentChapterIndex(CHAPTERS, 2000)).toBeNull()
    expect(currentChapterIndex([], 10)).toBeNull()
  })
})

describe('chapterProgress', () => {
  const chapter: Chapter = { start_seconds: 100, end_seconds: 200, title: 'Game' }

  it('is the fraction of the chapter played', () => {
    expect(chapterProgress(chapter, 150)).toBe(0.5)
  })

  it('is kept between 0 and 1', () => {
    expect(chapterProgress(chapter, 50)).toBe(0)
    expect(chapterProgress(chapter, 250)).toBe(1)
  })
})
