import { useState, type FormEvent } from 'react'
import type { UpdateChannelRequest } from '@/api/client'
import type { ChannelListItem } from '@/api/types'
import {
  channelSettingsChanges,
  lowersVideoLimit,
  type ChannelSettingsForm,
} from '@/lib/channelSettings'
import { VideoQualityField } from './VideoQualityField'
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'

interface EditChannelDialogProps {
  channel: ChannelListItem
  open: boolean
  onOpenChange: (open: boolean) => void
  /** Sends the changes; rejects to keep the dialog open with the error. */
  onSave: (changes: UpdateChannelRequest) => Promise<void>
}

export function EditChannelDialog({ channel, open, onOpenChange, onSave }: EditChannelDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="gap-5 p-6 sm:max-w-md">
        <DialogTitle>Edit {channel.name} settings</DialogTitle>
        <DialogDescription className="sr-only">
          Change the video quality and video limit of {channel.name}.
        </DialogDescription>
        {/* The form unmounts with the dialog, so every open starts from the
            channel's current settings. */}
        <EditChannelForm channel={channel} onSave={onSave} onClose={() => onOpenChange(false)} />
      </DialogContent>
    </Dialog>
  )
}

interface EditChannelFormProps {
  channel: ChannelListItem
  onSave: (changes: UpdateChannelRequest) => Promise<void>
  onClose: () => void
}

function EditChannelForm({ channel, onSave, onClose }: EditChannelFormProps) {
  const [form, setForm] = useState<ChannelSettingsForm>({
    quality: channel.quality,
    video_limit: String(channel.video_limit),
  })

  const setField = (field: keyof ChannelSettingsForm) => (event: { target: { value: string } }) => {
    const value = event.target.value
    setForm((prev) => ({ ...prev, [field]: value }))
  }

  const [error, setError] = useState<Error | null>(null)
  const [submitting, setSubmitting] = useState(false)
  const lowersLimit = lowersVideoLimit(channel.video_limit, form.video_limit)
  const changesQuality = form.quality !== channel.quality

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    const changes = channelSettingsChanges(channel, form)
    if (Object.keys(changes).length === 0) {
      onClose()
      return
    }
    setSubmitting(true)
    setError(null)
    try {
      await onSave(changes)
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
        <VideoQualityField
          id="edit-channel-quality"
          value={form.quality}
          onChange={setField('quality')}
        />
        {changesQuality && (
          <p className="text-xs text-muted-foreground">
            The new quality applies to new videos only. Videos already downloaded keep their current
            quality.
          </p>
        )}
      </div>
      <div className="flex flex-col gap-1.5">
        <Label htmlFor="edit-channel-video-limit">Video limit</Label>
        <Input
          id="edit-channel-video-limit"
          type="number"
          min="1"
          max="1000"
          step="1"
          value={form.video_limit}
          onChange={setField('video_limit')}
          aria-describedby={lowersLimit ? 'edit-channel-video-limit-warning' : undefined}
          required
        />
        {lowersLimit && (
          <p id="edit-channel-video-limit-warning" className="text-xs text-destructive">
            The next sync deletes the downloaded videos of this channel beyond the{' '}
            {Number(form.video_limit)} newest.
          </p>
        )}
      </div>

      {error && <p className="text-sm text-destructive">{error.message}</p>}
      <Button type="submit" disabled={submitting} className="self-start">
        {submitting ? 'Saving…' : 'Save'}
      </Button>
    </form>
  )
}
