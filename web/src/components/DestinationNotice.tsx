import type { ReactNode } from 'react'

interface DestinationNoticeProps {
  tone: 'info' | 'error'
  leading?: ReactNode
  children: ReactNode
}

/**
 * The confirmation line under an add dialog's "Save to" list, stating where
 * videos will go or why they can't. The location itself is changed through
 * the list above, so the notice offers no action of its own. `leading`, when
 * given, is rendered before the text.
 */
export function DestinationNotice({ tone, leading, children }: DestinationNoticeProps) {
  const color = tone === 'error' ? 'text-destructive' : 'text-muted-foreground'
  return (
    <p className={`mt-1 min-w-0 text-xs leading-relaxed ${color}`} data-testid="destination-notice">
      {leading}
      {children}
    </p>
  )
}

/** A destination path, set in the notice's monospace style. */
export function DestinationPath({ children }: { children: ReactNode }) {
  return <code className="wrap-anywhere font-mono text-foreground/80">{children}</code>
}
