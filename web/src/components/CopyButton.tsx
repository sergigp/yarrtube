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
    await copyToClipboard(text)
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

/**
 * Copies `text`: with the Clipboard API where the browser offers it (HTTPS or
 * localhost), else through a hidden text field, since yarrtube is often
 * served over plain HTTP on a home network.
 */
async function copyToClipboard(text: string): Promise<void> {
  // `navigator.clipboard` is undefined outside secure contexts.
  if (navigator.clipboard) {
    await navigator.clipboard.writeText(text)
    return
  }
  const field = document.createElement('textarea')
  field.value = text
  field.setAttribute('readonly', '')
  field.style.position = 'fixed'
  field.style.opacity = '0'
  document.body.appendChild(field)
  field.select()
  document.execCommand('copy')
  field.remove()
}
