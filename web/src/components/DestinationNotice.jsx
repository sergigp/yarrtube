/**
 * The helper line under an add dialog's main field, stating where videos
 * will go or why they can't. `onChange`, when given, renders a "change"
 * action after the text.
 */
export function DestinationNotice({ tone, children, onChange }) {
  const color = tone === 'error' ? 'text-destructive' : 'text-muted-foreground'
  return (
    <p
      className={`mt-1 min-w-0 text-xs leading-relaxed ${color}`}
      data-testid="destination-notice"
    >
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
export function DestinationPath({ children }) {
  return <code className="wrap-anywhere font-mono text-foreground/80">{children}</code>
}
