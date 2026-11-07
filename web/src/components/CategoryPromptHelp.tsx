import { CircleHelp } from 'lucide-react'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'

/**
 * A "?" next to a category's AI prompt that opens the guide to writing it. A
 * popover rather than a tooltip: it opens on click, so it works on touch screens.
 */
export function CategoryPromptHelp() {
  return (
    // Modal: its own scroll lock replaces the dialog's, which would swallow the wheel.
    <Popover modal>
      <PopoverTrigger asChild>
        <button
          type="button"
          aria-label="About AI prompt"
          className="rounded-full text-muted-foreground hover:text-foreground"
        >
          <CircleHelp className="size-4" />
        </button>
      </PopoverTrigger>
      <PopoverContent
        align="start"
        collisionPadding={16}
        className="max-h-[min(70vh,var(--radix-popover-content-available-height))] w-[min(28rem,calc(100vw-2rem))] overflow-y-auto"
      >
        <CategoryPromptGuide />
      </PopoverContent>
    </Popover>
  )
}

const BAD_PROMPTS = [
  { prompt: 'Chess', reason: 'Too vague: it says nothing about what is out.' },
  { prompt: 'Videos I would enjoy', reason: "Only you know that: a video's details can't tell." },
]

const TIPS = [
  "Name what is close but doesn't belong.",
  'Use words a title, chapter or summary would contain.',
  'Say whether one part of a video is enough.',
]

/** How to describe a category: a good example, bad ones and why, and tips. */
function CategoryPromptGuide() {
  return (
    <section aria-label="AI prompt guide" className="flex flex-col gap-4 text-sm">
      <p className="text-muted-foreground">
        Each video is checked against this description, so say what the videos are about.
      </p>
      <div role="group" aria-labelledby="category-prompt-good">
        <h3 id="category-prompt-good" className="font-semibold text-foreground">
          A good prompt
        </h3>
        <div className="mt-1 flex flex-col gap-1 rounded-md bg-muted px-3 py-2">
          <p>Competitive chess: games, openings, endgames and tournaments.</p>
          <p>Belongs: game analysis, opening guides, one chess game in a longer stream.</p>
          <p>Does not belong: other board games, chess.com drama, a passing mention of chess.</p>
        </div>
      </div>
      <div role="group" aria-labelledby="category-prompt-bad">
        <h3 id="category-prompt-bad" className="font-semibold text-foreground">
          Bad prompts
        </h3>
        <dl className="mt-1 flex flex-col gap-1">
          {BAD_PROMPTS.map((bad) => (
            <div key={bad.prompt}>
              <dt className="font-mono text-foreground">{bad.prompt}</dt>
              <dd className="text-muted-foreground">{bad.reason}</dd>
            </div>
          ))}
        </dl>
      </div>
      <div role="group" aria-labelledby="category-prompt-tips">
        <h3 id="category-prompt-tips" className="font-semibold text-foreground">
          Tips
        </h3>
        <ul className="mt-1 list-disc pl-4 text-muted-foreground">
          {TIPS.map((tip) => (
            <li key={tip}>{tip}</li>
          ))}
        </ul>
      </div>
    </section>
  )
}
