import type { ReactNode } from 'react'

interface DestinationNoticeProps {
  tone: 'info' | 'error'
  leading?: ReactNode
  children: ReactNode
  onChange?: () => void
}

/**
 * The helper line under an add dialog's main field, stating where videos
 * will go or why they can't. `leading`, when given, is rendered before the
 * text. `onChange`, when given, renders a "change" action after the text.
 */
export function DestinationNotice({ tone, leading, children, onChange }: DestinationNoticeProps) {
  const color = tone === 'error' ? 'text-destructive' : 'text-muted-foreground'
  return (
    <p className={`mt-1 min-w-0 text-xs leading-relaxed ${color}`} data-testid="destination-notice">
      {leading}
      {children}
      {onChange && (
        <>
          {' '}
          <button
            type="button"
            className="text-primary underline underline-offset-2"
            onClick={onChange}
          >
            change
          </button>
        </>
      )}
    </p>
  )
}

/** A destination path, set in the notice's monospace style. */
export function DestinationPath({ children }: { children: ReactNode }) {
  return <code className="wrap-anywhere font-mono text-foreground/80">{children}</code>
}
