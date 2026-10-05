import { Menu } from 'lucide-react'
import { Button } from '@/components/ui/button'

interface MobileNavToggleProps {
  open: boolean
  onToggle: () => void
  /** Id of the element the toggle shows and hides. */
  controls: string
}

export function MobileNavToggle({ onToggle, controls }: MobileNavToggleProps) {
  return (
    <Button
      variant="ghost"
      size="icon"
      className="md:hidden"
      onClick={onToggle}
      aria-label="Open menu"
      aria-controls={controls}
    >
      <Menu className="size-5" />
    </Button>
  )
}
