import { useState } from 'react'
import { cn } from '@/lib/utils'

const PLACEHOLDER =
  'bg-[repeating-linear-gradient(135deg,transparent,transparent_8px,color-mix(in_oklab,var(--border)_65%,transparent)_8px,color-mix(in_oklab,var(--border)_65%,transparent)_9px)] bg-muted'

/**
 * A video/channel thumbnail that falls back to the diagonal-hatch
 * placeholder pattern on missing or broken image URLs, instead of a
 * browser broken-image icon.
 */
export function Thumbnail({ src, alt = '', className }) {
  const [broken, setBroken] = useState(false)

  if (!src || broken) {
    return <span className={cn(PLACEHOLDER, className)} />
  }

  return <img src={src} alt={alt} className={className} onError={() => setBroken(true)} />
}
