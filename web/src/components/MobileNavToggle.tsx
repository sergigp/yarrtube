import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'

interface MobileNavToggleProps {
  open: boolean
  onToggle: () => void
  /** Id of the element the toggle shows and hides. */
  controls: string
}

const BAR =
  'absolute top-1/2 left-0.5 -mt-px h-0.5 w-4 rounded-full bg-current transition-[translate,rotate,opacity] duration-200 ease-in-out motion-reduce:transition-none'

/** The header's menu button: a hamburger that turns into an X while open. */
export function MobileNavToggle({ open, onToggle, controls }: MobileNavToggleProps) {
  return (
    <Button
      variant="ghost"
      size="icon"
      className="md:hidden"
      onClick={onToggle}
      aria-label={open ? 'Close menu' : 'Open menu'}
      aria-expanded={open}
      aria-controls={controls}
    >
      <span aria-hidden="true" className="relative block size-5">
        <span className={cn(BAR, open ? 'rotate-45' : '-translate-y-1.5')} />
        <span className={cn(BAR, open && 'opacity-0')} />
        <span className={cn(BAR, open ? '-rotate-45' : 'translate-y-1.5')} />
      </span>
    </Button>
  )
}
