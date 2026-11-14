import type { ReactNode } from 'react'
import { Check } from 'lucide-react'

/**
 * A settings section's title and description, with "Saved" beside the title
 * while `saved`, and an optional action (e.g. a switch) at its trailing edge.
 */
export function PageHeader({
  title,
  description,
  saved,
  action,
}: {
  title: string
  description?: ReactNode
  saved: boolean
  action?: ReactNode
}) {
  return (
    <div className="flex items-start justify-between gap-4">
      <div className="flex flex-col gap-1.5">
        <div className="flex flex-wrap items-center gap-x-4 gap-y-1">
          <h1 className="font-heading text-2xl font-bold text-foreground">{title}</h1>
          <p role="status" className="flex items-center gap-1 text-sm text-primary">
            {saved && (
              <>
                <Check className="size-4" aria-hidden="true" />
                Saved
              </>
            )}
          </p>
        </div>
        {description && <p className="text-sm text-muted-foreground">{description}</p>}
      </div>
      {action}
    </div>
  )
}
