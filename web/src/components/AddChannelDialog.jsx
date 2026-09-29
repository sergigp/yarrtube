import { useCallback, useState } from 'react'
import { createChannel } from '../api'
import { useChannelPreview, useChannels, useInvalidateLibrary } from '../queries'
import { useDebouncedValue } from '../useDebouncedValue'
import { deriveChannelPathSegment } from '../channelHandle'
import { channelNoticeLead } from '../channelNotice'
import { DestinationNotice, DestinationPath } from './DestinationNotice'
import { LocationField } from './LocationField'
import { Thumbnail } from './Thumbnail'
import { VideoQualityField } from './VideoQualityField'
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Button } from '@/components/ui/button'

const emptyForm = { channel: '', quality: 'high', video_limit: '3' }
const emptyLocation = { path: '', destination: '', valid: false }
const LOOKUP_DEBOUNCE_MS = 400

/** YouTube treats `@Name` and `@name` as the same channel. */
function sameHandle(a, b) {
  return a.toLowerCase() === b.toLowerCase()
}

/** The first case that applies wins: an error blocks, so it outranks the destination. */
function ChannelNotice({ lookingUp, preview, tracked, videoLimit, location, onChange }) {
  if (lookingUp) {
    return <DestinationNotice tone="info">Looking up channel…</DestinationNotice>
  }
  if (preview.error) {
    return <DestinationNotice tone="error">{preview.error.message}</DestinationNotice>
  }
  if (tracked) {
    return <DestinationNotice tone="error">Already added as “{tracked.name}”</DestinationNotice>
  }
  const destination = <DestinationPath>{location.destination}</DestinationPath>
  if (location.occupiedBy) {
    return (
      <DestinationNotice tone="error" onChange={onChange}>
        {destination} is already used by {location.occupiedBy}. Choose a different folder.
      </DestinationNotice>
    )
  }
  if (preview.data && location.destination) {
    const avatar = (
      <Thumbnail
        src={preview.data.avatar_url}
        className="mr-1.5 inline-block size-4 rounded-full object-cover align-text-bottom"
      />
    )
    return (
      <DestinationNotice tone="info" leading={avatar} onChange={onChange}>
        {channelNoticeLead(videoLimit, preview.data.title)} {destination}
      </DestinationNotice>
    )
  }
  return null
}

export function AddChannelDialog({ open, onOpenChange }) {
  const [form, setForm] = useState(emptyForm)
  const [location, setLocation] = useState(emptyLocation)
  const [advancedOpen, setAdvancedOpen] = useState(false)
  const [error, setError] = useState(null)
  const [submitting, setSubmitting] = useState(false)
  const invalidateLibrary = useInvalidateLibrary()
  const entered = form.channel.trim()
  const debounced = useDebouncedValue(entered, LOOKUP_DEBOUNCE_MS)
  const preview = useChannelPreview(debounced)
  // Until the lookup has caught up with the field, whatever the preview holds
  // belongs to an earlier value.
  const lookingUp = debounced !== entered || preview.isFetching
  const channels = useChannels()
  const tracked =
    preview.data && channels.data?.find((channel) => sameHandle(channel.id, preview.data.id))

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

  const canSubmit =
    Boolean(preview.data) && !tracked && !lookingUp && location.valid && !submitting

  const handleSubmit = async (event) => {
    event.preventDefault()
    if (!canSubmit) {
      return
    }
    setSubmitting(true)
    setError(null)
    try {
      await createChannel({
        channel: form.channel,
        quality: form.quality,
        video_limit: Number(form.video_limit),
        path: location.path,
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
        <DialogTitle>Add channel</DialogTitle>

        <form className="flex flex-col gap-4" onSubmit={handleSubmit}>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="add-channel-handle">Channel Handle or URL</Label>
            <Input
              id="add-channel-handle"
              type="text"
              value={form.channel}
              onChange={setField('channel')}
              placeholder="@somechannel"
              required
            />
            {/* Helper text under the field, not a block of its own: it
                confirms the defaults without competing with the form. */}
            {entered && (
              <ChannelNotice
                lookingUp={lookingUp}
                preview={preview}
                tracked={tracked}
                videoLimit={form.video_limit}
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
                mode="channel"
                nameSource={deriveChannelPathSegment(form.channel)}
                onChange={handleLocationChange}
              />
              <VideoQualityField
                id="add-channel-quality"
                value={form.quality}
                onChange={setField('quality')}
              />
              <div className="flex flex-col gap-1.5">
                <Label htmlFor="add-channel-video-limit">Video Limit</Label>
                <Input
                  id="add-channel-video-limit"
                  type="number"
                  min="1"
                  max="1000"
                  step="1"
                  value={form.video_limit}
                  onChange={setField('video_limit')}
                  required
                />
              </div>
            </div>
          </div>

          {error && <p className="text-sm text-destructive">{error.message}</p>}
          <Button type="submit" disabled={!canSubmit} className="self-start">
            {submitting ? 'Creating…' : 'Create Channel'}
          </Button>
        </form>
      </DialogContent>
    </Dialog>
  )
}
