import { useState, type ReactNode } from 'react'
import { ChevronRight } from 'lucide-react'
import { cn } from '@/lib/utils'

/**
 * "Prompt": a disclosure, closed each time the page opens, whose header says
 * whether the prompt is the shipped one ("default") or not ("edited").
 */
export function PromptDisclosure({
  isDefault,
  children,
}: {
  isDefault: boolean
  children: ReactNode
}) {
  const [open, setOpen] = useState(false)
  return (
    <div className="flex flex-col gap-3">
      <button
        type="button"
        aria-expanded={open}
        onClick={() => setOpen(!open)}
        className="flex items-center gap-1 self-start text-sm font-medium text-muted-foreground hover:text-foreground"
      >
        <ChevronRight
          className={cn('size-4 transition-transform', open && 'rotate-90')}
          aria-hidden="true"
        />
        Prompt
        <span className={cn('font-normal', !isDefault && 'text-amber-700 dark:text-amber-400')}>
          {isDefault ? ' · default' : ' · edited'}
        </span>
      </button>
      {open && children}
    </div>
  )
}
