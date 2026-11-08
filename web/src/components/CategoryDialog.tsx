import { useId, useRef, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { Check, ListVideo, Plus, RotateCcw, Search, X } from 'lucide-react'
import {
  avatarMediaUrl,
  createCategory,
  deleteCategoryAiVerdicts,
  setCategoryMembers,
  updateCategory,
} from '@/api/client'
import {
  useCategories,
  useCategorizationAvailable,
  useChannels,
  useInvalidateLibrary,
  usePlaylists,
} from '@/api/queries'
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
import { defaultColor } from '@/lib/categoryColors'
import { changedDetails, type CategoryFields } from '@/lib/categoryForm'
import { errorMessage } from '@/lib/errorMessage'
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group'
import { CategoryColorPicker } from './CategoryColorPicker'
import { CategoryPromptHelp } from './CategoryPromptHelp'
import { ConfirmDialog } from './ConfirmDialog'
import { Thumbnail } from './Thumbnail'

interface CategoryDialogProps {
  /** Absent when creating. */
  category?: Category
  open: boolean
  onOpenChange: (open: boolean) => void
}

const NO_MEMBERS: CategoryMembers = { channels: [], playlists: [] }

/**
 * A category's name, color, AI prompt and every tracked source as a chip,
 * pressed when in it; `category` absent to create one.
 */
export function CategoryDialog({ category, open, onOpenChange }: CategoryDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      {/* Mounted only while open, so every open starts from the saved sources. */}
      {open && <CategoryDialogContent category={category} onClose={() => onOpenChange(false)} />}
    </Dialog>
  )
}

/**
 * The form and the save controls. Editing only stages a change;
 * Save sends what changed, making the category's sources exactly the pressed
 * chips in one request. Closing with staged changes asks to discard them first.
 */
function CategoryDialogContent({ category, onClose }: { category?: Category; onClose: () => void }) {
  const { data: channels } = useChannels()
  const { data: playlists } = usePlaylists()
  const { data: categories } = useCategories()
  const invalidateLibrary = useInvalidateLibrary()
  const navigate = useNavigate()
  const saved = category ? membersOf(category) : NO_MEMBERS
  const [staged, setStaged] = useState(saved)
  const withPrompt = useCategorizationAvailable()
  // What the fields started from, or last saved here. Not the live category: a
  // refetch while open must not make an untouched field look edited.
  const [base, setBase] = useState<CategoryFields>({
    name: category?.name ?? '',
    color: category?.color ?? '',
    prompt: category?.ai_prompt ?? '',
  })
  const [name, setName] = useState(base.name)
  const [chosenColor, setChosenColor] = useState<string | null>(null)
  const [prompt, setPrompt] = useState(base.prompt)
  // Until one is picked, a new category takes the least used color, once loaded.
  const color =
    chosenColor ||
    base.color ||
    defaultColor((categories?.categories ?? []).map((existing) => existing.color))
  // Saved by a save whose sources then failed: they can no longer be discarded.
  const [detailsSaved, setDetailsSaved] = useState(false)
  const [search, setSearch] = useState('')
  const [filter, setFilter] = useState<SourceFilter>('all')
  const [confirmingReevaluate, setConfirmingReevaluate] = useState(false)
  const reevaluated = category?.ai_verdict_count ?? 0
  const reevaluatedVideos = reevaluated === 1 ? '1 video' : `${reevaluated} videos`
  const [confirmingDiscard, setConfirmingDiscard] = useState(false)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const nameRef = useRef<HTMLInputElement>(null)
  const chips = sourceChips(channels, playlists)
  const allChips = [...chips.channels, ...chips.playlists]
  const shown = (kind: SourceChip[]) => visibleChips(kind, staged, search, filter)
  const shownChannels = shown(chips.channels)
  const shownPlaylists = shown(chips.playlists)
  const inCount = allChips.filter((chip) => isIn(staged, chip.source)).length
  const summary = stagedSummary(saved, staged)
  const sourcesChanged = summary.added + summary.removed > 0
  const details = changedDetails(
    base,
    { name, color: chosenColor ?? base.color, prompt },
    { withPrompt },
  )
  const detailsChanged = Object.keys(details).length > 0
  const promptChanged = 'ai_prompt' in details
  const dirty = sourcesChanged || detailsChanged
  const categoryName = category?.name ?? 'this category'
  const canSave = name.trim() !== '' && (!category || dirty) && !saving
  const toggle = (source: SourceRef) => setStaged((current) => toggled(current, source))

  const requestClose = () => (dirty ? setConfirmingDiscard(true) : onClose())
  const guardDismiss = (event: Event) => {
    if (dirty) {
      event.preventDefault()
      setConfirmingDiscard(true)
    }
  }

  const create = async () => {
    setSaving(true)
    setError(null)
    const aiPrompt = withPrompt ? prompt.trim() : ''
    try {
      const created = await createCategory({
        name: name.trim(),
        color,
        ...(aiPrompt !== '' && { ai_prompt: aiPrompt }),
        ...staged,
      })
      invalidateLibrary()
      onClose()
      navigate(`/categories/${encodeURIComponent(created.id)}`)
    } catch (err) {
      setError(errorMessage(err))
    } finally {
      setSaving(false)
    }
  }

  const save = async () => {
    if (!category) {
      return create()
    }
    setSaving(true)
    setError(null)
    let detailsStored = detailsSaved
    try {
      if (detailsChanged) {
        await updateCategory(category.id, details)
        detailsStored = true
        setDetailsSaved(true)
        setBase({
          name: name.trim(),
          color: chosenColor ?? base.color,
          prompt: withPrompt ? prompt.trim() : base.prompt,
        })
      }
      if (sourcesChanged) {
        await setCategoryMembers(category.id, staged)
      }
      onClose()
    } catch (err) {
      setError(
        detailsStored
          ? `Your other changes were saved, but the sources weren't: ${errorMessage(err)}`
          : errorMessage(err),
      )
    } finally {
      // Also after a failure: the details may be saved even if the sources aren't.
      invalidateLibrary()
      setSaving(false)
    }
  }

  return (
    <DialogContent
      showCloseButton={false}
      onOpenAutoFocus={(event) => {
        event.preventDefault()
        nameRef.current?.focus()
      }}
      onEscapeKeyDown={guardDismiss}
      onInteractOutside={guardDismiss}
      className="flex max-h-[calc(100dvh-2rem)] w-full flex-col gap-0 overflow-hidden p-0 max-sm:h-dvh max-sm:max-h-dvh max-sm:max-w-full max-sm:rounded-none sm:h-[min(48rem,calc(100dvh-2rem))] sm:max-w-2xl"
    >
      <div className="flex items-start gap-3 px-6 pt-6 pb-4">
        <div className="flex min-w-0 flex-1 flex-col gap-1.5">
          <DialogTitle>{category ? `Edit ${category.name} settings` : 'Add category'}</DialogTitle>
          <DialogDescription>
            Pick the channels and playlists whose videos show in {categoryName}.
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
      <form
        className="flex min-h-0 flex-1 flex-col"
        onSubmit={(event) => {
          event.preventDefault()
          if (canSave) {
            void save()
          }
        }}
      >
        {/* Everything between the header and the save controls scrolls together, so
            the prompt can never push them out of a short dialog. */}
        <div className="min-h-0 flex-1 overflow-y-auto">
          <div className="flex flex-col gap-4 px-6 pb-4">
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="category-name">Name</Label>
              <Input
                ref={nameRef}
                id="category-name"
                type="text"
                value={name}
                onChange={(event) => setName(event.target.value)}
              />
            </div>
            {/* Enter on a swatch or the hue slider would submit the form. */}
            <div
              className="flex flex-col gap-1.5"
              onKeyDown={(event) => {
                if (event.key === 'Enter') {
                  event.preventDefault()
                }
              }}
            >
              <span className="text-sm leading-none font-medium">Color</span>
              <CategoryColorPicker value={color} onChange={setChosenColor} />
            </div>
          </div>
          {withPrompt && (
            <div className="flex flex-col gap-1.5 px-6 pb-4">
              <div className="flex items-center gap-1.5">
                <Label htmlFor="category-ai-prompt">AI prompt</Label>
                <CategoryPromptHelp />
              </div>
              <textarea
                id="category-ai-prompt"
                rows={6}
                value={prompt}
                onChange={(event) => setPrompt(event.target.value)}
                aria-describedby="category-ai-prompt-hint"
                className="w-full resize-none rounded-lg border border-input bg-transparent px-2.5 py-2 text-sm outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 dark:bg-input/30"
              />
              <p id="category-ai-prompt-hint" className="text-xs text-muted-foreground">
                Describe the videos that belong in this category.
              </p>
              {reevaluated > 0 && (
                // An unsaved prompt would be ignored: the AI asks with the saved one.
                <div className="flex flex-col gap-1">
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    className="self-start"
                    disabled={promptChanged}
                    onClick={() => setConfirmingReevaluate(true)}
                  >
                    <RotateCcw />
                    Re-evaluate {reevaluatedVideos}
                  </Button>
                  {promptChanged && (
                    <p className="text-xs text-muted-foreground">
                      Save the prompt first to re-evaluate with it.
                    </p>
                  )}
                </div>
              )}
            </div>
          )}
          <div className="sticky top-0 z-10 flex flex-col gap-3 border-b border-border bg-popover px-6 pb-4">
            <div className="relative">
              <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                type="search"
                aria-label="Search channels and playlists"
                placeholder="Search channels and playlists"
                className="pl-8"
                value={search}
                onChange={(event) => setSearch(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key !== 'Enter') {
                    return
                  }
                  // Never saves; picks the only match, so typing a name and Enter adds it.
                  event.preventDefault()
                  const matches = [...shownChannels, ...shownPlaylists]
                  if (matches.length === 1) {
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
              <FilterItem value="in" label={`In ${categoryName}`} count={inCount} />
              <FilterItem
                value="out"
                label={`Not in ${categoryName}`}
                count={allChips.length - inCount}
              />
            </ToggleGroup>
          </div>
          <div className="flex flex-col gap-5 px-6 py-4">
            {channels?.length === 0 && playlists?.length === 0 && (
              <p className="py-6 text-center text-sm text-muted-foreground">
                No channels or playlists to add yet. Track one from the sidebar, then come back.
              </p>
            )}
            <ChipGroup title="Channels" chips={shownChannels} staged={staged} onToggle={toggle} />
            <ChipGroup title="Playlists" chips={shownPlaylists} staged={staged} onToggle={toggle} />
          </div>
        </div>
        <div className="flex flex-wrap items-center justify-end gap-3 border-t border-border px-6 py-4">
          {error && <p className="w-full text-sm text-destructive">{error}</p>}
          <p className="mr-auto text-sm text-muted-foreground">{summaryText(summary)}</p>
          <Button type="button" variant="outline" onClick={requestClose}>
            Cancel
          </Button>
          <Button type="submit" disabled={!canSave}>
            {category ? (saving ? 'Saving…' : 'Save') : saving ? 'Creating…' : 'Create'}
          </Button>
        </div>
      </form>
      {category && (
        <ConfirmDialog
          open={confirmingReevaluate}
          onOpenChange={setConfirmingReevaluate}
          title={`Re-evaluate ${reevaluatedVideos}?`}
          description={`The AI will evaluate ${reevaluated === 1 ? 'this video' : `these ${reevaluatedVideos}`} for ${category.name} again. Categories you set by hand stay.`}
          confirmLabel="Re-evaluate"
          pendingLabel="Re-evaluating…"
          onConfirm={async () => {
            await deleteCategoryAiVerdicts(category.id)
            await invalidateLibrary()
          }}
        />
      )}
      <ConfirmDialog
        open={confirmingDiscard}
        onOpenChange={setConfirmingDiscard}
        title="Discard changes?"
        description={
          !category
            ? "This category won't be created."
            : detailsSaved && !detailsChanged
              ? `Your other changes are already saved. Your changes to the sources in ${categoryName} won't be saved.`
              : `Your changes to ${categoryName} won't be saved.`
        }
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
