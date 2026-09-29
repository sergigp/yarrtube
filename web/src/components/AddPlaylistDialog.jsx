import { useCallback, useState } from 'react'
import { createPlaylist } from '../api'
import { useInvalidateLibrary, usePlaylistPreview, usePlaylists } from '../queries'
import { useLookup } from '../useLookup'
import { playlistNoticeLead } from '../playlistNotice'
import { DestinationNotice, DestinationPath } from './DestinationNotice'
import { lookupNotice } from './lookupNotice'
import { LocationField } from './LocationField'
import { VideoQualityField } from './VideoQualityField'
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Button } from '@/components/ui/button'

const emptyForm = { playlist: '', quality: 'high' }
const emptyLocation = { path: '', destination: '', valid: false }

function sameId(a, b) {
  return a === b
}

/** The first case that applies wins: an error blocks, so it outranks the destination. */
function PlaylistNotice({ lookup, location, onChange }) {
  const pending = lookupNotice('playlist', lookup)
  if (pending) {
    return pending
  }
  const { preview } = lookup
  const destination = <DestinationPath>{location.destination}</DestinationPath>
  if (location.occupiedBy) {
    return (
      <DestinationNotice tone="error" onChange={onChange}>
        {destination} is already used by {location.occupiedBy}. Choose a different folder.
      </DestinationNotice>
    )
  }
  if (preview.data && location.destination) {
    return (
      <DestinationNotice tone="info" onChange={onChange}>
        {playlistNoticeLead(preview.data.video_count, preview.data.title)} {destination}
      </DestinationNotice>
    )
  }
  return null
}

export function AddPlaylistDialog({ open, onOpenChange }) {
  const [form, setForm] = useState(emptyForm)
  const [location, setLocation] = useState(emptyLocation)
  const [advancedOpen, setAdvancedOpen] = useState(false)
  const [error, setError] = useState(null)
  const [submitting, setSubmitting] = useState(false)
  const invalidateLibrary = useInvalidateLibrary()
  const entered = form.playlist.trim()
  const playlists = usePlaylists()
  const lookup = useLookup(entered, usePlaylistPreview, playlists.data, sameId)

  const resetAll = () => {
    setForm(emptyForm)
    setLocation(emptyLocation)
    setAdvancedOpen(false)
    setError(null)
  }

  const handleOpenChange = (nextOpen) => {
    if (!nextOpen) {
      resetAll()
    }
    onOpenChange(nextOpen)
  }

  const setField = (field) => (event) => {
    const value = event.target.value
    setForm((prev) => ({ ...prev, [field]: value }))
  }

  // Stable: `LocationField` reports the composed destination from an effect.
  const handleLocationChange = useCallback((next) => setLocation(next), [])

  const canSubmit = lookup.addable && location.valid && !submitting

  const handleSubmit = async (event) => {
    event.preventDefault()
    if (!canSubmit) {
      return
    }
    setSubmitting(true)
    setError(null)
    try {
      await createPlaylist({
        playlist: form.playlist,
        path: location.path,
        quality: form.quality,
      })
      invalidateLibrary()
      resetAll()
      onOpenChange(false)
    } catch (err) {
      setError(err)
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      {/* `DialogContent` is centered with no height cap, so without the
          max-height an expanded "Advanced options" can push the submit button
          off a short viewport. `overflow-x-hidden` is not redundant: capping
          one axis makes CSS compute the other to `auto`, which would let a
          long path scroll the dialog sideways. */}
      <DialogContent
        className="max-h-[calc(100dvh-2rem)] overflow-x-hidden overflow-y-auto sm:max-w-md"
        onPointerDownOutside={(event) => event.preventDefault()}
        onInteractOutside={(event) => event.preventDefault()}
      >
        <DialogTitle>Add playlist</DialogTitle>

        <form className="flex flex-col gap-4" onSubmit={handleSubmit}>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="add-playlist-id">Playlist ID or URL</Label>
            <Input
              id="add-playlist-id"
              type="text"
              value={form.playlist}
              onChange={setField('playlist')}
              required
            />
            {/* Helper text under the field, as in the channel dialog. */}
            {entered && (
              <PlaylistNotice
                lookup={lookup}
                location={location}
                onChange={() => setAdvancedOpen(true)}
              />
            )}
          </div>

          <div className="border-t border-border pt-3">
            <button
              type="button"
              className="text-sm text-muted-foreground transition-colors hover:text-foreground"
              onClick={() => setAdvancedOpen((prev) => !prev)}
              aria-expanded={advancedOpen}
            >
              {advancedOpen ? '▾' : '▸'} Advanced options
            </button>

            {/* Kept mounted while collapsed: `LocationField` holds the
                browsed parent and edited folder name, and reports the
                destination the notice above shows. */}
            <div className="mt-3 flex flex-col gap-4" hidden={!advancedOpen}>
              <LocationField
                mode="playlist"
                nameSource={lookup.preview.data?.title ?? ''}
                onChange={handleLocationChange}
              />
              <VideoQualityField
                id="add-playlist-quality"
                value={form.quality}
                onChange={setField('quality')}
              />
            </div>
          </div>

          {error && <p className="text-sm text-destructive">{error.message}</p>}
          <Button type="submit" disabled={!canSubmit} className="self-start">
            {submitting ? 'Creating…' : 'Create Playlist'}
          </Button>
        </form>
      </DialogContent>
    </Dialog>
  )
}
