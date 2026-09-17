import { useState } from 'react'
import { Info } from 'lucide-react'
import { createPlaylist, createChannel } from '../api'
import { slugify } from '../slugify'
import { deriveChannelPathSegment } from '../channelHandle'
import {
  Dialog,
  DialogContent,
  DialogTitle,
} from '@/components/ui/dialog'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { Button } from '@/components/ui/button'

const emptyPlaylistForm = { playlist: '', name: '', path: '', quality: 'high' }
const emptyChannelForm = { channel: '', path: '', quality: 'high', video_limit: '3' }

function isPathConflictError(err) {
  return typeof err?.message === 'string' && err.message.includes('is already used by another playlist')
}

function VideoQualityField({ id, value, onChange }) {
  return (
    <div className="flex flex-col gap-1.5">
      <Label htmlFor={id}>
        Video quality
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <button
                type="button"
                className="inline-flex size-4 items-center justify-center rounded-full text-muted-foreground"
                aria-label="About video quality"
              >
                <Info className="size-3.5" />
              </button>
            </TooltipTrigger>
            <TooltipContent>
              Controls the resolution videos are downloaded at. A lower resolution reduces
              storage use.
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
      </Label>
      <Select value={value} onValueChange={(next) => onChange({ target: { value: next } })}>
        <SelectTrigger id={id} className="w-full">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="high">High</SelectItem>
          <SelectItem value="mid">Mid</SelectItem>
          <SelectItem value="low">Low</SelectItem>
        </SelectContent>
      </Select>
    </div>
  )
}

export function AddDialog({ open, onOpenChange }) {
  const [mode, setMode] = useState('playlist')
  const [playlistForm, setPlaylistForm] = useState(emptyPlaylistForm)
  const [channelForm, setChannelForm] = useState(emptyChannelForm)
  const [pathEdited, setPathEdited] = useState(false)
  const [advancedOpen, setAdvancedOpen] = useState(false)
  const [error, setError] = useState(null)
  const [submitting, setSubmitting] = useState(false)

  const resetAll = () => {
    setMode('playlist')
    setPlaylistForm(emptyPlaylistForm)
    setChannelForm(emptyChannelForm)
    setPathEdited(false)
    setAdvancedOpen(false)
    setError(null)
  }

  const handleOpenChange = (nextOpen) => {
    if (!nextOpen) {
      resetAll()
    }
    onOpenChange(nextOpen)
  }

  const switchMode = (nextMode) => {
    setMode(nextMode)
    setPathEdited(false)
    setError(null)
  }

  const setPlaylistField = (field) => (event) => {
    const value = event.target.value
    setPlaylistForm((prev) => {
      const next = { ...prev, [field]: value }
      if (field === 'name' && !pathEdited) {
        next.path = value ? `playlists/${slugify(value)}` : ''
      }
      return next
    })
    if (field === 'path') {
      setPathEdited(true)
    }
  }

  const setChannelField = (field) => (event) => {
    const value = event.target.value
    setChannelForm((prev) => {
      const next = { ...prev, [field]: value }
      if (field === 'channel' && !pathEdited) {
        const segment = deriveChannelPathSegment(value)
        next.path = segment ? `channels/${slugify(segment)}` : ''
      }
      return next
    })
    if (field === 'path') {
      setPathEdited(true)
    }
  }

  const handleSubmit = async (event) => {
    event.preventDefault()
    setSubmitting(true)
    setError(null)
    try {
      if (mode === 'playlist') {
        await createPlaylist(playlistForm)
      } else {
        await createChannel({
          channel: channelForm.channel,
          quality: channelForm.quality,
          video_limit: Number(channelForm.video_limit),
          path: channelForm.path,
        })
      }
      resetAll()
      onOpenChange(false)
    } catch (err) {
      setError(err)
      if (isPathConflictError(err)) {
        setAdvancedOpen(true)
      }
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent
        className="sm:max-w-md"
        onPointerDownOutside={(event) => event.preventDefault()}
        onInteractOutside={(event) => event.preventDefault()}
      >
        <DialogTitle>Add</DialogTitle>

        <Tabs value={mode} onValueChange={switchMode}>
          <TabsList className="w-full rounded-full p-1">
            <TabsTrigger value="playlist" className="rounded-full">
              Playlist
            </TabsTrigger>
            <TabsTrigger value="channel" className="rounded-full">
              Channel
            </TabsTrigger>
          </TabsList>
        </Tabs>

        <form className="flex flex-col gap-4" onSubmit={handleSubmit}>
          {mode === 'playlist' ? (
            <>
              <div className="flex flex-col gap-1.5">
                <Label htmlFor="add-playlist-id">Playlist ID or URL</Label>
                <Input
                  id="add-playlist-id"
                  type="text"
                  value={playlistForm.playlist}
                  onChange={setPlaylistField('playlist')}
                  required
                />
              </div>
              <div className="flex flex-col gap-1.5">
                <Label htmlFor="add-playlist-name">Name</Label>
                <Input
                  id="add-playlist-name"
                  type="text"
                  value={playlistForm.name}
                  onChange={setPlaylistField('name')}
                  required
                />
              </div>
            </>
          ) : (
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="add-channel-handle">Channel Handle or URL</Label>
              <Input
                id="add-channel-handle"
                type="text"
                value={channelForm.channel}
                onChange={setChannelField('channel')}
                placeholder="@somechannel"
                required
              />
            </div>
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

            {advancedOpen && (
              <div className="mt-3 flex flex-col gap-4">
                {mode === 'playlist' ? (
                  <>
                    <div className="flex flex-col gap-1.5">
                      <Label htmlFor="add-playlist-path">Path</Label>
                      <Input
                        id="add-playlist-path"
                        type="text"
                        value={playlistForm.path}
                        onChange={setPlaylistField('path')}
                        required
                      />
                    </div>
                    <VideoQualityField
                      id="add-playlist-quality"
                      value={playlistForm.quality}
                      onChange={setPlaylistField('quality')}
                    />
                  </>
                ) : (
                  <>
                    <div className="flex flex-col gap-1.5">
                      <Label htmlFor="add-channel-path">Path</Label>
                      <Input
                        id="add-channel-path"
                        type="text"
                        value={channelForm.path}
                        onChange={setChannelField('path')}
                        required
                      />
                    </div>
                    <VideoQualityField
                      id="add-channel-quality"
                      value={channelForm.quality}
                      onChange={setChannelField('quality')}
                    />
                    <div className="flex flex-col gap-1.5">
                      <Label htmlFor="add-channel-video-limit">Video Limit</Label>
                      <Input
                        id="add-channel-video-limit"
                        type="number"
                        min="1"
                        step="1"
                        value={channelForm.video_limit}
                        onChange={setChannelField('video_limit')}
                        required
                      />
                    </div>
                  </>
                )}
              </div>
            )}
          </div>

          {error && <p className="text-sm text-destructive">{error.message}</p>}
          <Button type="submit" disabled={submitting} className="self-start">
            {submitting ? 'Creating…' : mode === 'playlist' ? 'Create Playlist' : 'Create Channel'}
          </Button>
        </form>
      </DialogContent>
    </Dialog>
  )
}
