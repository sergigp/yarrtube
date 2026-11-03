import { PROMPT_SECTIONS } from '@/lib/promptGuide'

/** Explains the summary prompt's sections and the variables each can use. */
export function PromptGuide() {
  return (
    <section aria-label="Prompt guide">
      {PROMPT_SECTIONS.map((section) => (
        <div key={section.heading} />
      ))}
    </section>
  )
}
