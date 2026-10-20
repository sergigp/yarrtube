import { useState } from 'react'
import { Pencil } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'

interface EditableTitleProps {
  name: string
  onRename: (name: string) => Promise<void>
}

/**
 * A heading with a "Rename" pencil that turns it into an in-place field:
 * Enter or leaving the field saves, Escape cancels.
 */
export function EditableTitle({ name, onRename }: EditableTitleProps) {
  const [draft, setDraft] = useState<string | null>(null)

  if (draft === null) {
    return (
      <div className="group flex min-w-0 items-center gap-1">
        <h2 className="truncate font-heading text-lg font-semibold text-foreground md:text-xl">
          {name}
        </h2>
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                aria-label="Rename"
                className="shrink-0 text-muted-foreground md:opacity-0 md:group-hover:opacity-100 md:focus-visible:opacity-100"
                onClick={() => setDraft(name)}
              >
                <Pencil />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Rename</TooltipContent>
          </Tooltip>
        </TooltipProvider>
      </div>
    )
  }

  const save = async () => {
    setDraft(null)
    if (draft !== name) {
      await onRename(draft)
    }
  }

  return (
    <Input
      aria-label="Name"
      value={draft}
      autoFocus
      onFocus={(event) => event.target.select()}
      onChange={(event) => setDraft(event.target.value)}
      onBlur={save}
      onKeyDown={(event) => {
        if (event.key === 'Enter') {
          event.preventDefault()
          void save()
        } else if (event.key === 'Escape') {
          event.preventDefault()
          setDraft(null)
        }
      }}
      className="h-auto min-w-0 px-2 py-0.5 font-heading text-lg font-semibold md:text-xl"
    />
  )
}
