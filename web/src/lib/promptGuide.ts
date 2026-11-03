/** A variable a prompt section can use, and what it is replaced by. */
export interface PromptVariable {
  name: string
  description: string
}

/** A section of the summary prompt, as the daemon accepts it. */
export interface PromptSectionGuide {
  heading: string
  purpose: string
  variables: PromptVariable[]
}

const LANG: PromptVariable = {
  name: 'lang',
  description: "The video's language, e.g. English or Spanish: summaries are written in it.",
}
const TITLE: PromptVariable = {
  name: 'title',
  description: "The video's original YouTube title.",
}
const MINUTES: PromptVariable = {
  name: 'minutes',
  description: "The video's length in minutes.",
}
const CHAPTERS: PromptVariable = {
  name: 'chapters',
  description: 'The "chapters" snippet with every chapter, or nothing when the video has none.',
}
const CHAPTER_LIST: PromptVariable = {
  name: 'chapter_list',
  description: 'One line per chapter set by the uploader: its start time and title.',
}
const PART: PromptVariable = { name: 'part', description: "The part's number, from 1." }
const SPAN: PromptVariable = {
  name: 'span',
  description: 'The part\'s start and end time in the video, e.g. 0:00–25:30.',
}

/**
 * Every section of the summary prompt, in document order, with the variables
 * the daemon lets each use (mirrors `SummaryPromptSection` in the daemon).
 */
export const PROMPT_SECTIONS: PromptSectionGuide[] = [
  {
    heading: 'system',
    purpose: 'Instructions for a video that fits the context length in one call.',
    variables: [LANG],
  },
  {
    heading: 'user',
    purpose: 'The message with the whole video, sent with "system".',
    variables: [
      TITLE,
      MINUTES,
      CHAPTERS,
      { name: 'transcript', description: "The video's whole transcript." },
      LANG,
    ],
  },
  {
    heading: 'chapters',
    purpose: 'Inserted as {chapters} in "user" when the video has chapters.',
    variables: [CHAPTER_LIST],
  },
  {
    heading: 'map system',
    purpose: 'Instructions for summarizing one part of a video too long for one call.',
    variables: [LANG],
  },
  {
    heading: 'map user',
    purpose: 'The message with one part of a long video, sent with "map system".',
    variables: [
      TITLE,
      MINUTES,
      PART,
      { name: 'parts', description: 'How many parts the video was cut into.' },
      SPAN,
      {
        ...CHAPTERS,
        description: 'The "map chapters" snippet with the chapters in this part, or nothing.',
      },
      { name: 'transcript', description: "This part's transcript." },
      LANG,
    ],
  },
  {
    heading: 'map chapters',
    purpose: 'Inserted as {chapters} in "map user" when the part has chapters.',
    variables: [CHAPTER_LIST],
  },
  {
    heading: 'reduce system',
    purpose: "Instructions for combining a long video's part summaries into one summary.",
    variables: [LANG],
  },
  {
    heading: 'reduce user',
    purpose: 'The message with every part summary, sent with "reduce system".',
    variables: [
      TITLE,
      MINUTES,
      CHAPTERS,
      {
        name: 'part_summaries',
        description: 'Every part summary in order, each written with "reduce part".',
      },
      LANG,
    ],
  },
  {
    heading: 'reduce part',
    purpose: 'How each part summary is written into {part_summaries}.',
    variables: [PART, SPAN, { name: 'summary', description: "The part's summary." }],
  },
]
