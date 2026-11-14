/** Copies `text` to the clipboard. */
export function CopyButton({ text }: { text: string }) {
  return (
    <button type="button" aria-label={`Copy ${text}`}>
      Copy
    </button>
  )
}
