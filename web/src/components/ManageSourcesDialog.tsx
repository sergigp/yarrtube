import { useId, useRef, useState } from 'react'
import { Check, ListVideo, Plus, Search, X } from 'lucide-react'
import { avatarMediaUrl, setCategoryMembers } from '@/api/client'
import { useChannels, useInvalidateLibrary, usePlaylists } from '@/api/queries'
import type { Category, CategoryMembers } from '@/api/types'
import {
  isIn,
  membersOf,
  sourceChips,
  stagedSummary,
  toggled,
  visibleChips,
  type SourceChip,
  type SourceFilter,
  type SourceRef,
} from '@/lib/categories'
import { errorMessage } from '@/lib/errorMessage'
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group'
import { ConfirmDialog } from './ConfirmDialog'
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
      {/* Mounted only while open, so every open starts from the saved sources. */}
      {open && <ManageSourcesContent category={category} onClose={() => onOpenChange(false)} />}
    </Dialog>
  )
}

/**
 * The chips and save controls. Toggling only stages a change; Save makes the
 * category's sources exactly the pressed chips in one request. Closing with
 * staged changes asks to discard them first.
 */
function ManageSourcesContent({ category, onClose }: { category: Category; onClose: () => void }) {
  const { data: channels } = useChannels()
  const { data: playlists } = usePlaylists()
  const invalidateLibrary = useInvalidateLibrary()
  const saved = membersOf(category)
  const [staged, setStaged] = useState(saved)
  const [search, setSearch] = useState('')
  const [filter, setFilter] = useState<SourceFilter>('all')
  const [confirmingDiscard, setConfirmingDiscard] = useState(false)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const searchRef = useRef<HTMLInputElement>(null)
  const chips = sourceChips(channels, playlists)
  const allChips = [...chips.channels, ...chips.playlists]
  const shown = (kind: SourceChip[]) => visibleChips(kind, staged, search, filter)
  const shownChannels = shown(chips.channels)
  const shownPlaylists = shown(chips.playlists)
  const inCount = allChips.filter((chip) => isIn(staged, chip.source)).length
  const summary = stagedSummary(saved, staged)
  const dirty = summary.added + summary.removed > 0
  const toggle = (source: SourceRef) => setStaged((current) => toggled(current, source))

  const requestClose = () => (dirty ? setConfirmingDiscard(true) : onClose())
  const guardDismiss = (event: Event) => {
    if (dirty) {
      event.preventDefault()
      setConfirmingDiscard(true)
    }
  }

  const save = async () => {
    setSaving(true)
    setError(null)
    try {
      await setCategoryMembers(category.id, staged)
      invalidateLibrary()
      onClose()
    } catch (err) {
      setError(errorMessage(err))
    } finally {
      setSaving(false)
    }
  }

  return (
    <DialogContent
      showCloseButton={false}
      onOpenAutoFocus={(event) => {
        event.preventDefault()
        searchRef.current?.focus()
      }}
      onEscapeKeyDown={guardDismiss}
      onInteractOutside={guardDismiss}
      className="flex max-h-[calc(100dvh-2rem)] w-full flex-col gap-0 overflow-hidden p-0 max-sm:h-dvh max-sm:max-h-dvh max-sm:max-w-full max-sm:rounded-none sm:h-[min(40rem,calc(100dvh-2rem))] sm:max-w-2xl"
    >
      <div className="flex items-start gap-3 px-6 pt-6 pb-4">
        <div className="flex min-w-0 flex-1 flex-col gap-1.5">
          <DialogTitle>Manage sources in {category.name}</DialogTitle>
          <DialogDescription>
            Pick the channels and playlists whose videos show in {category.name}.
          </DialogDescription>
        </div>
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          aria-label="Close"
          className="-mt-2 -mr-2"
          onClick={requestClose}
        >
          <X />
        </Button>
      </div>
      <div className="flex flex-col gap-3 px-6 pb-4">
        <div className="relative">
          <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            ref={searchRef}
            type="search"
            aria-label="Search channels and playlists"
            placeholder="Search channels and playlists"
            className="pl-8"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            onKeyDown={(event) => {
              // Enter picks the only match, so typing a name and Enter adds it.
              const matches = [...shownChannels, ...shownPlaylists]
              if (event.key === 'Enter' && matches.length === 1) {
                event.preventDefault()
                toggle(matches[0]!.source)
              }
            }}
          />
        </div>
        <ToggleGroup
          type="single"
          variant="outline"
          size="sm"
          aria-label="Show"
          className="flex-wrap"
          value={filter}
          onValueChange={(value) => {
            if (value) {
              setFilter(value as SourceFilter)
            }
          }}
        >
          <FilterItem value="all" label="All" />
          <FilterItem value="in" label={`In ${category.name}`} count={inCount} />
          <FilterItem
            value="out"
            label={`Not in ${category.name}`}
            count={allChips.length - inCount}
          />
        </ToggleGroup>
      </div>
      <div className="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto border-t border-border px-6 py-4">
        {channels?.length === 0 && playlists?.length === 0 && (
          <p className="py-6 text-center text-sm text-muted-foreground">
            No channels or playlists to add yet. Track one from the sidebar, then come back.
          </p>
        )}
        <ChipGroup title="Channels" chips={shownChannels} staged={staged} onToggle={toggle} />
        <ChipGroup title="Playlists" chips={shownPlaylists} staged={staged} onToggle={toggle} />
      </div>
      <div className="flex flex-wrap items-center justify-end gap-3 border-t border-border px-6 py-4">
        {error && <p className="w-full text-sm text-destructive">{error}</p>}
        <p className="mr-auto text-sm text-muted-foreground">{summaryText(summary)}</p>
        <Button type="button" variant="outline" onClick={requestClose}>
          Cancel
        </Button>
        <Button type="button" disabled={!dirty || saving} onClick={save}>
          {saving ? 'Saving…' : 'Save'}
        </Button>
      </div>
      <ConfirmDialog
        open={confirmingDiscard}
        onOpenChange={setConfirmingDiscard}
        title="Discard changes?"
        description={`Your changes to the sources in ${category.name} won't be saved.`}
        cancelLabel="Keep editing"
        confirmLabel="Discard"
        onConfirm={onClose}
      />
    </DialogContent>
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

/** A state filter pill, with how many sources it holds. */
function FilterItem({ value, label, count }: { value: SourceFilter; label: string; count?: number }) {
  return (
    <ToggleGroupItem
      value={value}
      className="rounded-full px-3 font-normal data-[state=on]:border-primary/40 data-[state=on]:bg-primary/10 data-[state=on]:text-primary"
    >
      <span className="max-w-40 truncate">{label}</span>
      {count !== undefined && <span className="text-xs text-muted-foreground">{count}</span>}
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
