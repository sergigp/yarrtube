import { Check, Lock } from 'lucide-react'
import type { VideoCategoryChoice } from '@/api/types'
import { useSetVideoCategory, useVideoCategories } from '@/api/queries'
import { useTheme } from '@/hooks/useTheme'
import { inkShade, softTint } from '@/lib/categoryColors'
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog'
import { cn } from '@/lib/utils'

interface VideoCategoriesDialogProps {
  /** The stored copy's `record_id`. */
  recordId: string
  /** The video's title, the dialog's title. */
  title: string
  /** The channel or playlist the copy comes from, named on locked chips. */
  sourceName: string
  open: boolean
  onOpenChange: (open: boolean) => void
}

/**
 * Every category as a chip in its color, filled when the video belongs to it.
 * Chips from the video's channel or playlist are locked; chips the AI decided
 * carry an "AI" mark.
 */
export function VideoCategoriesDialog({
  recordId,
  title,
  sourceName,
  open,
  onOpenChange,
}: VideoCategoriesDialogProps) {
  const { data, error } = useVideoCategories(recordId, open)
  const setVideoCategory = useSetVideoCategory()

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogTitle>{title}</DialogTitle>
        <DialogDescription>
          Click a category to add the video to it or take it out.
        </DialogDescription>
        {error && (
          <p className="text-sm text-destructive">Failed to load categories: {error.message}</p>
        )}
        {data && (
          <div role="toolbar" aria-label="Categories" className="flex flex-wrap gap-1.5">
            {data.categories.map((choice) => (
              <ChoiceChip
                key={choice.id}
                choice={choice}
                sourceName={sourceName}
                onToggle={() => setVideoCategory(recordId, choice.id, !choice.belongs)}
              />
            ))}
          </div>
        )}
      </DialogContent>
    </Dialog>
  )
}

function ChoiceChip({
  choice,
  sourceName,
  onToggle,
}: {
  choice: VideoCategoryChoice
  sourceName: string
  onToggle: () => void
}) {
  const theme = useTheme()
  const locked = choice.from === 'source'
  const byAi = choice.from === 'ai'
  return (
    <button
      type="button"
      aria-pressed={choice.belongs}
      aria-label={
        locked ? `${choice.name}, from ${sourceName}` : byAi ? `${choice.name}, decided by AI` : undefined
      }
      disabled={locked}
      className={cn(
        'flex items-center gap-1.5 rounded-full border py-1 pr-3 pl-2 text-xs font-medium transition-colors',
        choice.belongs
          ? 'border-foreground/10'
          : 'border-border text-muted-foreground hover:text-foreground',
        locked && 'cursor-not-allowed',
      )}
      style={
        choice.belongs
          ? { background: softTint(choice.color, theme), color: inkShade(choice.color, theme) }
          : undefined
      }
      onClick={onToggle}
    >
      {locked ? (
        <Lock className="size-3" />
      ) : choice.belongs ? (
        <Check className="size-3" />
      ) : (
        <span
          aria-hidden
          className="size-3 rounded-full border border-foreground/10"
          style={{ background: choice.color }}
        />
      )}
      {choice.name}
      {locked && <span className="font-normal opacity-75">From {sourceName}</span>}
      {byAi && (
        <span className="rounded-sm border border-current px-1 text-[0.625rem] leading-3.5 opacity-75">
          AI
        </span>
      )}
    </button>
  )
}
