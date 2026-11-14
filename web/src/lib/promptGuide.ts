/** A variable the summary prompt can use, and what it is replaced by. */
export interface PromptVariable {
  name: string
  description: string
}

/**
 * Every variable the summary prompt can use, once each, in the order a
 * reader meets them (the daemon's `SummaryPromptSection` decides which
 * section may use which).
 */
export const PROMPT_VARIABLES: PromptVariable[] = [
  {
    name: 'lang',
    description: "The video's language, e.g. English or Spanish: summaries are written in it.",
  },
  { name: 'title', description: "The video's original YouTube title." },
  { name: 'minutes', description: "The video's length in minutes." },
  {
    name: 'transcript',
    description: "The transcript: the whole video's, or one part's when a long video is cut into parts.",
  },
  {
    name: 'chapters',
    description: "The video's chapters, or nothing when it has none.",
  },
  { name: 'chapter_list', description: 'One line per chapter: its start time and title.' },
  { name: 'part', description: "The part's number, from 1." },
  { name: 'parts', description: 'How many parts a long video was cut into.' },
  { name: 'span', description: "The part's start and end time in the video, e.g. 0:00–25:30." },
  { name: 'part_summaries', description: 'Every part summary, in order.' },
  { name: 'summary', description: "One part's summary." },
]
