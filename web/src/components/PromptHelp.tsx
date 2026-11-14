import { CircleHelp } from 'lucide-react'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { PROMPT_VARIABLES } from '@/lib/promptGuide'

/**
 * A "?" next to the prompt that opens the list of variables it can use. A
 * popover rather than a tooltip, so it opens on click and can scroll.
 */
export function PromptHelp() {
  return (
    <Popover>
      <PopoverTrigger asChild>
        <button
          type="button"
          aria-label="About Prompt"
          className="rounded-full text-muted-foreground hover:text-foreground"
        >
          <CircleHelp className="size-4" />
        </button>
      </PopoverTrigger>
      <PopoverContent
        align="start"
        className="max-h-[70vh] w-[min(36rem,calc(100vw-2rem))] overflow-y-auto"
      >
        <PromptGuide />
      </PopoverContent>
    </Popover>
  )
}

/** The summary prompt's variables, each with what it is replaced by. */
function PromptGuide() {
  return (
    <section aria-label="Prompt guide" className="text-sm">
      <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1.5">
        {PROMPT_VARIABLES.map((variable) => (
          <div key={variable.name} className="contents">
            <dt className="font-mono text-foreground">{`{${variable.name}}`}</dt>
            <dd className="text-muted-foreground">{variable.description}</dd>
          </div>
        ))}
      </dl>
    </section>
  )
}
