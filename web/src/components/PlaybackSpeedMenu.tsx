import { ChevronDown } from 'lucide-react'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { formatPlaybackSpeed, PLAYBACK_SPEEDS } from '@/lib/playbackSpeed'
import { cn } from '@/lib/utils'

interface PlaybackSpeedMenuProps {
  rate: number
  onRateChange: (rate: number) => void
  disabled: boolean
  className?: string
}

/** A menu of playback speeds, labelled with the current one. */
export function PlaybackSpeedMenu({
  rate,
  onRateChange,
  disabled,
  className,
}: PlaybackSpeedMenuProps) {
  return (
    <DropdownMenu modal={false}>
      <DropdownMenuTrigger
        className={cn(
          'flex h-7 shrink-0 items-center gap-0.5 rounded-md px-1.5 text-sm text-muted-foreground tabular-nums transition-colors hover:bg-secondary hover:text-foreground disabled:pointer-events-none disabled:opacity-50 data-[state=open]:bg-secondary data-[state=open]:text-foreground',
          className,
        )}
        aria-label="Playback speed"
        disabled={disabled}
      >
        {formatPlaybackSpeed(rate)}
        <ChevronDown className="size-3.5" />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-auto min-w-24">
        <DropdownMenuRadioGroup
          value={String(rate)}
          onValueChange={(value) => onRateChange(Number(value))}
        >
          {PLAYBACK_SPEEDS.map((speed) => (
            <DropdownMenuRadioItem key={speed} value={String(speed)}>
              {formatPlaybackSpeed(speed)}
            </DropdownMenuRadioItem>
          ))}
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
