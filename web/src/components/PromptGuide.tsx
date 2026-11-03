import { PROMPT_SECTIONS } from '@/lib/promptGuide'

/** Explains the summary prompt's sections and the variables each can use. */
export function PromptGuide() {
  return (
    <section
      aria-label="Prompt guide"
      className="flex flex-col gap-4 rounded-lg border border-border bg-muted/40 p-4 text-sm"
    >
      <p className="text-muted-foreground">
        The prompt is split into sections, each starting with a <code>## heading</code> line. A
        video that fits the context length is summarized in one call with <code>system</code> and{' '}
        <code>user</code>. A longer one is cut into parts: each part is summarized with{' '}
        <code>map system</code> and <code>map user</code>, then the part summaries are combined
        with <code>reduce system</code> and <code>reduce user</code>. <code>chapters</code>,{' '}
        <code>map chapters</code> and <code>reduce part</code> are snippets inserted into those
        messages. Each section can use only its own variables, written as <code>{'{name}'}</code>.
      </p>
      {PROMPT_SECTIONS.map((section) => (
        <div key={section.heading} role="group" aria-label={section.heading}>
          <h3 className="font-mono font-semibold text-foreground">## {section.heading}</h3>
          <p className="text-muted-foreground">{section.purpose}</p>
          <dl className="mt-1 grid grid-cols-[auto_1fr] gap-x-3 gap-y-0.5">
            {section.variables.map((variable) => (
              <div key={variable.name} className="contents">
                <dt className="font-mono text-foreground">{`{${variable.name}}`}</dt>
                <dd className="text-muted-foreground">{variable.description}</dd>
              </div>
            ))}
          </dl>
        </div>
      ))}
    </section>
  )
}
