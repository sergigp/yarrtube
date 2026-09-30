import { useState, type FormEvent } from 'react'
import { createChannel } from '@/api/client'
import { useChannelPreview, useChannels, useInvalidateLibrary } from '@/api/queries'
import type { ChannelListItem, ChannelPreview } from '@/api/types'
import { useLookup, type Lookup } from '@/hooks/useLookup'
import { useSaveLocation, type LocationValue } from '@/hooks/useSaveLocation'
import { deriveChannelPathSegment } from '@/lib/channelHandle'
import { channelNoticeLead } from '@/lib/noticeLead'
import { DestinationNotice, DestinationPath } from './DestinationNotice'
import { lookupNotice } from './lookupNotice'
import { FolderNameField } from './FolderNameField'
import { SaveToField } from './SaveToField'
import { Thumbnail } from './Thumbnail'
import { VideoQualityField } from './VideoQualityField'
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Button } from '@/components/ui/button'

/** YouTube treats `@Name` and `@name` as the same channel. */
function sameHandle(a: string, b: string): boolean {
  return a.toLowerCase() === b.toLowerCase()
}

interface ChannelNoticeProps {
  lookup: Lookup<ChannelPreview, ChannelListItem>
  videoLimit: string
  location: LocationValue
}

/** The first case that applies wins: an error blocks, so it outranks the destination. */
function ChannelNotice({ lookup, videoLimit, location }: ChannelNoticeProps) {
  const pending = lookupNotice('channel', lookup)
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
    const avatar = (
      <Thumbnail
        src={preview.data.avatar_url}
        className="mr-1.5 inline-block size-4 rounded-full object-cover align-text-bottom"
      />
    )
    return (
      <DestinationNotice tone="info" leading={avatar}>
        {channelNoticeLead(videoLimit, preview.data.title)} {destination}
      </DestinationNotice>
    )
  }
  return null
}

interface AddChannelDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function AddChannelDialog({ open, onOpenChange }: AddChannelDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
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
        {/* The form unmounts with the dialog, so every open starts a fresh
            session: fields, browsed parent and folder-name edits included. */}
        <AddChannelForm onClose={() => onOpenChange(false)} />
      </DialogContent>
    </Dialog>
  )
}

function AddChannelForm({ onClose }: { onClose: () => void }) {
  const [form, setForm] = useState({ channel: '', quality: 'high', video_limit: '3' })
  const [advancedOpen, setAdvancedOpen] = useState(false)
  const [error, setError] = useState<Error | null>(null)
  const [submitting, setSubmitting] = useState(false)
  const invalidateLibrary = useInvalidateLibrary()
  const entered = form.channel.trim()
  const channels = useChannels()
  const lookup = useLookup(entered, useChannelPreview, channels.data, sameHandle)
  const location = useSaveLocation('channel', deriveChannelPathSegment(form.channel))

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
      await createChannel({
        channel: form.channel,
        quality: form.quality,
        video_limit: Number(form.video_limit),
        path: location.value.path,
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
      </div>

      <SaveToField location={location} />

      {entered && (
        <ChannelNotice lookup={lookup} videoLimit={form.video_limit} location={location.value} />
      )}

      <div className="border-t border-border pt-3">
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
        <div className="mt-3 flex flex-col gap-4" hidden={!advancedOpen}>
          <FolderNameField location={location} id="add-channel-folder-name" />
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
  )
}
