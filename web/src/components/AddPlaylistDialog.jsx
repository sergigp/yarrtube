import { useCallback, useState } from 'react'
import { createPlaylist } from '../api'
import { useInvalidateLibrary } from '../queries'
import { LocationField } from './LocationField'
import { VideoQualityField } from './VideoQualityField'
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Button } from '@/components/ui/button'

const emptyForm = { playlist: '', name: '', quality: 'high' }
const emptyLocation = { path: '', valid: false }

export function AddPlaylistDialog({ open, onOpenChange }) {
  const [form, setForm] = useState(emptyForm)
  const [location, setLocation] = useState(emptyLocation)
  const [advancedOpen, setAdvancedOpen] = useState(false)
  const [error, setError] = useState(null)
  const [submitting, setSubmitting] = useState(false)
  const invalidateLibrary = useInvalidateLibrary()

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

  const handleSubmit = async (event) => {
    event.preventDefault()
    if (!location.valid) {
      return
    }
    setSubmitting(true)
    setError(null)
    try {
      await createPlaylist({
        playlist: form.playlist,
        name: form.name,
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
      {/* The location controls make this dialog tall enough to outgrow a short
          viewport, and `DialogContent` is centered with no height cap, so
          without the max-height the submit button can end up off-screen.
          `overflow-x-hidden` is not redundant: capping one axis makes CSS
          compute the other to `auto`, which would let a long path scroll the
          dialog sideways. */}
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
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="add-playlist-name">Name</Label>
            <Input
              id="add-playlist-name"
              type="text"
              value={form.name}
              onChange={setField('name')}
              required
            />
          </div>

          <LocationField mode="playlist" nameSource={form.name} onChange={handleLocationChange} />

          <div className="border-t border-border pt-3">
            <button
              type="button"
              className="text-sm text-muted-foreground transition-colors hover:text-foreground"
              onClick={() => setAdvancedOpen((prev) => !prev)}
              aria-expanded={advancedOpen}
            >
              {advancedOpen ? '▾' : '▸'} Advanced options
            </button>

            {advancedOpen && (
              <div className="mt-3 flex flex-col gap-4">
                <VideoQualityField
                  id="add-playlist-quality"
                  value={form.quality}
                  onChange={setField('quality')}
                />
              </div>
            )}
          </div>

          {error && <p className="text-sm text-destructive">{error.message}</p>}
          <Button type="submit" disabled={submitting || !location.valid} className="self-start">
            {submitting ? 'Creating…' : 'Create Playlist'}
          </Button>
        </form>
      </DialogContent>
    </Dialog>
  )
}
