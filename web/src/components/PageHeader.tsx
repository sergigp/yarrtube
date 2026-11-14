import type { ReactNode } from 'react'
import { Check } from 'lucide-react'

/** A settings section's title, with "Saved" beside it while `saved`. */
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
    <div className="flex items-center justify-between gap-4">
      <h1 className="font-heading text-xl font-semibold text-foreground">{title}</h1>
      {description}
      {action}
      <p role="status" className="flex items-center gap-1 text-sm text-muted-foreground">
        {saved && (
          <>
            <Check className="size-4" aria-hidden="true" />
            Saved
          </>
        )}
      </p>
    </div>
  )
}
