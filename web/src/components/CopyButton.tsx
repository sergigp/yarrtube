import { useEffect, useState } from 'react'
import { Button } from '@/components/ui/button'

const COPIED_SHOWN_MS = 2000

/** Copies `text` to the clipboard, reading "Copied" for 2 seconds after. */
export function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false)
  useEffect(() => {
    if (!copied) {
      return undefined
    }
    const timer = setTimeout(() => setCopied(false), COPIED_SHOWN_MS)
    return () => clearTimeout(timer)
  }, [copied])

  async function copy() {
    await navigator.clipboard.writeText(text)
    setCopied(true)
  }

  return (
    <Button
      variant="outline"
      size="sm"
      aria-label={`Copy ${text}`}
      className="shrink-0"
      onClick={() => void copy()}
    >
      {copied ? 'Copied' : 'Copy'}
    </Button>
  )
}
