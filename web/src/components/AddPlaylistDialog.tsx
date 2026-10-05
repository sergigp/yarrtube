import { useState, type FormEvent } from 'react'
import { createPlaylist } from '@/api/client'
import { useInvalidateLibrary, usePlaylistPreview, usePlaylists } from '@/api/queries'
import type { PlaylistListItem, PlaylistPreview } from '@/api/types'
import { useLookup, type Lookup } from '@/hooks/useLookup'
import { useSaveLocation, type LocationValue } from '@/hooks/useSaveLocation'
import { playlistNoticeLead } from '@/lib/noticeLead'
import { DestinationNotice, DestinationPath } from './DestinationNotice'
import { lookupNotice } from './lookupNotice'
import { FolderNameField } from './FolderNameField'
import { SaveToField } from './SaveToField'
import { VideoQualityField } from './VideoQualityField'
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Button } from '@/components/ui/button'

function sameId(a: string, b: string): boolean {
  return a === b
}

interface PlaylistNoticeProps {
  lookup: Lookup<PlaylistPreview, PlaylistListItem>
  location: LocationValue
}

/** The first case that applies wins: an error blocks, so it outranks the destination. */
function PlaylistNotice({ lookup, location }: PlaylistNoticeProps) {
  const pending = lookupNotice('playlist', lookup)
  if (pending) {
    return pending
  }
  const { preview } = lookup
  const destination = <DestinationPath>{location.destination}</DestinationPath>
  if (location.occupiedBy) {
    return (
      <DestinationNotice tone="error">
        {destination} is already used by {location.occupiedBy}. Choose a different folder.
      </DestinationNotice>
    )
  }
  if (preview.data && location.destination) {
    return (
      <DestinationNotice tone="info">
        {playlistNoticeLead(preview.data.video_count, preview.data.title)} {destination}
      </DestinationNotice>
    )
  }
  return null
}

interface AddPlaylistDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function AddPlaylistDialog({ open, onOpenChange }: AddPlaylistDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      {/* `DialogContent` is centered with no height cap, so without the
          max-height an expanded "Advanced options" can push the submit button
          off a short viewport. `overflow-x-hidden` is not redundant: capping
          one axis makes CSS compute the other to `auto`, which would let a
          long path scroll the dialog sideways. */}
      <DialogContent
        className="max-h-[calc(100dvh-2rem)] gap-5 overflow-x-hidden overflow-y-auto p-6 sm:max-w-lg"
        onPointerDownOutside={(event) => event.preventDefault()}
        onInteractOutside={(event) => event.preventDefault()}
      >
        <DialogTitle>Add playlist</DialogTitle>
        {/* The form unmounts with the dialog, so every open starts a fresh
            session: fields, browsed parent and folder-name edits included. */}
        <AddPlaylistForm onClose={() => onOpenChange(false)} />
      </DialogContent>
    </Dialog>
  )
}

function AddPlaylistForm({ onClose }: { onClose: () => void }) {
  const [form, setForm] = useState({ playlist: '', quality: 'high' })
  const [excludeFromHome, setExcludeFromHome] = useState(false)
  const [advancedOpen, setAdvancedOpen] = useState(false)
  const [error, setError] = useState<Error | null>(null)
  const [submitting, setSubmitting] = useState(false)
  const invalidateLibrary = useInvalidateLibrary()
  const entered = form.playlist.trim()
  const playlists = usePlaylists()
  const lookup = useLookup(entered, usePlaylistPreview, playlists.data, sameId)
  const location = useSaveLocation('playlist', lookup.preview.data?.title ?? '')

  const setField =
    (field: keyof typeof form) => (event: { target: { value: string } }) => {
      const value = event.target.value
      setForm((prev) => ({ ...prev, [field]: value }))
    }

  const canSubmit = lookup.addable && location.value.valid && !submitting

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (!canSubmit) {
      return
    }
    setSubmitting(true)
    setError(null)
    try {
      await createPlaylist({
        playlist: form.playlist,
        path: location.value.path,
        quality: form.quality,
        exclude_from_home: excludeFromHome,
      })
      location.remember()
      invalidateLibrary()
      onClose()
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)))
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <form className="flex min-w-0 flex-col gap-5" onSubmit={handleSubmit}>
      <div className="flex flex-col gap-1.5">
        <Label htmlFor="add-playlist-id">Playlist ID or URL</Label>
        <Input
          id="add-playlist-id"
          type="text"
          value={form.playlist}
          onChange={setField('playlist')}
          required
        />
      </div>

      <SaveToField location={location} />

      {entered && <PlaylistNotice lookup={lookup} location={location.value} />}

      <div className="min-w-0 border-t border-border pt-4">
        <button
          type="button"
          className="text-sm text-muted-foreground transition-colors hover:text-foreground"
          onClick={() => setAdvancedOpen((prev) => !prev)}
          aria-expanded={advancedOpen}
        >
          {advancedOpen ? '▾' : '▸'} Advanced options
        </button>

        {/* Kept mounted while collapsed: the folder-name field shows the
            auto-filled value the notice above composes the destination from. */}
        <div className="mt-4 flex min-w-0 flex-col gap-5" hidden={!advancedOpen}>
          <FolderNameField location={location} id="add-playlist-folder-name" />
          <VideoQualityField
            id="add-playlist-quality"
            value={form.quality}
            onChange={setField('quality')}
          />
          <div className="flex items-center gap-2">
            <input
              id="add-playlist-exclude-from-home"
              type="checkbox"
              className="size-4 accent-primary"
              checked={excludeFromHome}
              onChange={(event) => setExcludeFromHome(event.target.checked)}
            />
            <Label htmlFor="add-playlist-exclude-from-home">Exclude from home</Label>
          </div>
        </div>
      </div>

      {error && <p className="text-sm text-destructive">{error.message}</p>}
      <Button type="submit" disabled={!canSubmit} className="self-start">
        {submitting ? 'Creating…' : 'Create Playlist'}
      </Button>
    </form>
  )
}
