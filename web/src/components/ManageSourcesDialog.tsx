import { useId, useState } from 'react'
import { Check, ListVideo, Plus } from 'lucide-react'
import { avatarMediaUrl, setCategoryMembers } from '@/api/client'
import { useChannels, useInvalidateLibrary, usePlaylists } from '@/api/queries'
import type { Category, CategoryMembers } from '@/api/types'
import {
  isIn,
  membersOf,
  sourceChips,
  stagedSummary,
  toggled,
  type SourceChip,
  type SourceRef,
} from '@/lib/categories'
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group'
import { Thumbnail } from './Thumbnail'

interface ManageSourcesDialogProps {
  category: Category
  open: boolean
  onOpenChange: (open: boolean) => void
}

/** Every tracked channel and playlist as a chip, pressed when in the category. */
export function ManageSourcesDialog({ category, open, onOpenChange }: ManageSourcesDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="flex max-h-[calc(100dvh-2rem)] w-full flex-col gap-0 overflow-hidden p-0 max-sm:h-dvh max-sm:max-h-dvh max-sm:max-w-full max-sm:rounded-none sm:max-w-2xl">
        <div className="flex flex-col gap-1.5 px-6 pt-6 pb-4">
          <DialogTitle>Manage sources in {category.name}</DialogTitle>
          <DialogDescription>
            Pick the channels and playlists whose videos show in {category.name}.
          </DialogDescription>
        </div>
        {/* The body unmounts with the dialog, so every open starts from the saved sources. */}
        <ManageSourcesBody category={category} onClose={() => onOpenChange(false)} />
      </DialogContent>
    </Dialog>
  )
}

/**
 * The chips and save controls. Toggling only stages a change; Save makes the
 * category's sources exactly the pressed chips in one request.
 */
function ManageSourcesBody({ category, onClose }: { category: Category; onClose: () => void }) {
  const { data: channels } = useChannels()
  const { data: playlists } = usePlaylists()
  const invalidateLibrary = useInvalidateLibrary()
  const saved = membersOf(category)
  const [staged, setStaged] = useState(saved)
  const chips = sourceChips(channels, playlists)
  const toggle = (source: SourceRef) => setStaged((current) => toggled(current, source))

  const save = async () => {
    await setCategoryMembers(category.id, staged)
    invalidateLibrary()
    onClose()
  }

  return (
    <>
      <div className="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto border-t border-border px-6 py-4">
        <ChipGroup title="Channels" chips={chips.channels} staged={staged} onToggle={toggle} />
        <ChipGroup title="Playlists" chips={chips.playlists} staged={staged} onToggle={toggle} />
      </div>
      <div className="flex flex-wrap items-center justify-end gap-3 border-t border-border px-6 py-4">
        <p className="mr-auto text-sm text-muted-foreground">
          {summaryText(stagedSummary(saved, staged))}
        </p>
        <Button type="button" onClick={save}>
          Save
        </Button>
      </div>
    </>
  )
}

interface ChipGroupProps {
  title: string
  chips: SourceChip[]
  staged: CategoryMembers
  onToggle: (source: SourceRef) => void
}

/** One kind's chips; arrow keys move between them. */
function ChipGroup({ title, chips, staged, onToggle }: ChipGroupProps) {
  const headingId = useId()
  if (chips.length === 0) {
    return null
  }
  return (
    <section className="flex flex-col gap-2">
      <h3
        id={headingId}
        className="text-xs font-medium tracking-wide text-muted-foreground uppercase"
      >
        {title}
      </h3>
      <ToggleGroup
        type="multiple"
        aria-labelledby={headingId}
        variant="outline"
        className="w-full flex-wrap justify-start"
        value={chips.filter((chip) => isIn(staged, chip.source)).map(chipValue)}
        onValueChange={(values) => {
          const pressed = new Set(values)
          const changed = chips.find(
            (chip) => pressed.has(chipValue(chip)) !== isIn(staged, chip.source),
          )
          if (changed) {
            onToggle(changed.source)
          }
        }}
      >
        {chips.map((chip) => (
          <SourceChipItem key={chipValue(chip)} chip={chip} pressed={isIn(staged, chip.source)} />
        ))}
      </ToggleGroup>
    </section>
  )
}

function SourceChipItem({ chip, pressed }: { chip: SourceChip; pressed: boolean }) {
  return (
    <ToggleGroupItem
      value={chipValue(chip)}
      className="h-8 max-w-full gap-1.5 rounded-full pr-3 pl-1 font-normal text-muted-foreground hover:text-foreground data-[state=on]:border-primary/40 data-[state=on]:bg-primary/10 data-[state=on]:text-primary"
    >
      {chip.source.kind === 'channel' ? (
        <Thumbnail
          src={chip.avatarFilename ? avatarMediaUrl(chip.avatarFilename) : null}
          className="size-6 shrink-0 rounded-full object-cover"
        />
      ) : (
        <span className="flex size-6 shrink-0 items-center justify-center rounded-full bg-muted">
          <ListVideo className="size-3.5" />
        </span>
      )}
      <span className="truncate">{chip.name}</span>
      {pressed ? <Check className="size-3.5" /> : <Plus className="size-3.5" />}
    </ToggleGroupItem>
  )
}

/** The staged changes, such as "+2 added, −1 removed"; empty when there are none. */
function summaryText({ added, removed }: { added: number; removed: number }): string {
  return [added > 0 && `+${added} added`, removed > 0 && `−${removed} removed`]
    .filter(Boolean)
    .join(', ')
}

function chipValue(chip: SourceChip): string {
  return `${chip.source.kind}:${chip.source.id}`
}
