import { useCallback, useState } from 'react'
import { Info } from 'lucide-react'
import { createPlaylist, createChannel } from '../api'
import { deriveChannelPathSegment } from '../channelHandle'
import { LocationField } from './LocationField'
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

const emptyPlaylistForm = { playlist: '', name: '', quality: 'high' }
const emptyChannelForm = { channel: '', quality: 'high', video_limit: '3' }
const emptyLocation = { path: '', valid: false }

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
  const [location, setLocation] = useState(emptyLocation)
  const [advancedOpen, setAdvancedOpen] = useState(false)
  const [error, setError] = useState(null)
  const [submitting, setSubmitting] = useState(false)

  const resetAll = () => {
    setMode('playlist')
    setPlaylistForm(emptyPlaylistForm)
    setChannelForm(emptyChannelForm)
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

  const switchMode = (nextMode) => {
    setMode(nextMode)
    setError(null)
  }

  const setPlaylistField = (field) => (event) => {
    const value = event.target.value
    setPlaylistForm((prev) => ({ ...prev, [field]: value }))
  }

  const setChannelField = (field) => (event) => {
    const value = event.target.value
    setChannelForm((prev) => ({ ...prev, [field]: value }))
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
      if (mode === 'playlist') {
        await createPlaylist({
          playlist: playlistForm.playlist,
          name: playlistForm.name,
          path: location.path,
          quality: playlistForm.quality,
        })
      } else {
        await createChannel({
          channel: channelForm.channel,
          quality: channelForm.quality,
          video_limit: Number(channelForm.video_limit),
          path: location.path,
        })
      }
      resetAll()
      onOpenChange(false)
    } catch (err) {
      setError(err)
    } finally {
      setSubmitting(false)
    }
  }

  const nameSource =
    mode === 'playlist' ? playlistForm.name : deriveChannelPathSegment(channelForm.channel)

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

          {/* Keyed by mode: switching tabs starts the location over on that
              mode's default parent, with the folder name auto-filling again
              from the other tab's source field. */}
          <LocationField
            key={mode}
            mode={mode}
            nameSource={nameSource}
            onChange={handleLocationChange}
          />

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
                  <VideoQualityField
                    id="add-playlist-quality"
                    value={playlistForm.quality}
                    onChange={setPlaylistField('quality')}
                  />
                ) : (
                  <>
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
          <Button type="submit" disabled={submitting || !location.valid} className="self-start">
            {submitting ? 'Creating…' : mode === 'playlist' ? 'Create Playlist' : 'Create Channel'}
          </Button>
        </form>
      </DialogContent>
    </Dialog>
  )
}
