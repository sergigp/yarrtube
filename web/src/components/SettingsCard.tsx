import { useId, type ReactNode } from 'react'

/**
 * One group of settings: a card with a heading, an optional description and
 * an action beside the heading (e.g. the switch turning the group on), then
 * its fields. Exposed as a group named after its heading.
 */
export function SettingsCard({
  id,
  title,
  description,
  action,
  children,
}: {
  id?: string
  title: string
  description?: ReactNode
  action?: ReactNode
  children?: ReactNode
}) {
  const headingId = useId()
  return (
    <section
      id={id}
      role="group"
      aria-labelledby={headingId}
      className="flex flex-col gap-4 rounded-lg border border-border bg-card p-6 text-card-foreground"
    >
      <div className="flex items-start justify-between gap-6">
        <div className="flex flex-col gap-1">
          <h2 id={headingId} className="font-heading text-lg font-semibold text-foreground">
            {title}
          </h2>
          {description && <div className="text-sm text-muted-foreground">{description}</div>}
        </div>
        {action}
      </div>
      {children}
    </section>
  )
}
