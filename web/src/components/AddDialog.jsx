import { useState } from 'react'
import * as Dialog from '@radix-ui/react-dialog'
import * as Tooltip from '@radix-ui/react-tooltip'
import { createPlaylist, createChannel } from '../api'
import { slugify } from '../slugify'
import { deriveChannelPathSegment } from '../channelHandle'

const emptyPlaylistForm = { playlist: '', name: '', path: '', quality: 'high' }
const emptyChannelForm = { channel: '', path: '', quality: 'high', video_limit: '3' }

function isPathConflictError(err) {
  return typeof err?.message === 'string' && err.message.includes('is already used by another playlist')
}

function VideoQualityField({ id, value, onChange }) {
  return (
    <div className="form-row">
      <label htmlFor={id}>
        Video quality{' '}
        <Tooltip.Provider delayDuration={200}>
          <Tooltip.Root>
            <Tooltip.Trigger asChild>
              <button type="button" className="tooltip-trigger" aria-label="About video quality">
                ?
              </button>
            </Tooltip.Trigger>
            <Tooltip.Portal>
              <Tooltip.Content className="tooltip-content" sideOffset={5}>
                Controls the resolution videos are downloaded at. A lower resolution reduces
                storage use.
                <Tooltip.Arrow className="tooltip-arrow" />
              </Tooltip.Content>
            </Tooltip.Portal>
          </Tooltip.Root>
        </Tooltip.Provider>
      </label>
      <select id={id} value={value} onChange={onChange}>
        <option value="high">High</option>
        <option value="mid">Mid</option>
        <option value="low">Low</option>
      </select>
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
    <Dialog.Root open={open} onOpenChange={handleOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="dialog-overlay" />
        <Dialog.Content
          className="dialog-content"
          onPointerDownOutside={(event) => event.preventDefault()}
          onInteractOutside={(event) => event.preventDefault()}
        >
          <Dialog.Title className="dialog-title">Add</Dialog.Title>

          <div className="add-dialog-switcher tabs">
            <button
              type="button"
              className={mode === 'playlist' ? 'tab active' : 'tab'}
              onClick={() => switchMode('playlist')}
            >
              Playlist
            </button>
            <button
              type="button"
              className={mode === 'channel' ? 'tab active' : 'tab'}
              onClick={() => switchMode('channel')}
            >
              Channel
            </button>
          </div>

          <form className="create-playlist-form" onSubmit={handleSubmit}>
            {mode === 'playlist' ? (
              <>
                <div className="form-row">
                  <label htmlFor="add-playlist-id">Playlist ID or URL</label>
                  <input
                    id="add-playlist-id"
                    type="text"
                    value={playlistForm.playlist}
                    onChange={setPlaylistField('playlist')}
                    required
                  />
                </div>
                <div className="form-row">
                  <label htmlFor="add-playlist-name">Name</label>
                  <input
                    id="add-playlist-name"
                    type="text"
                    value={playlistForm.name}
                    onChange={setPlaylistField('name')}
                    required
                  />
                </div>
              </>
            ) : (
              <div className="form-row">
                <label htmlFor="add-channel-handle">Channel Handle or URL</label>
                <input
                  id="add-channel-handle"
                  type="text"
                  value={channelForm.channel}
                  onChange={setChannelField('channel')}
                  placeholder="@somechannel"
                  required
                />
              </div>
            )}

            <div className="advanced-options">
              <button
                type="button"
                className="advanced-options-toggle"
                onClick={() => setAdvancedOpen((prev) => !prev)}
                aria-expanded={advancedOpen}
              >
                {advancedOpen ? '▾' : '▸'} Advanced options
              </button>

              {advancedOpen && (
                <div className="advanced-options-content">
                  {mode === 'playlist' ? (
                    <>
                      <div className="form-row">
                        <label htmlFor="add-playlist-path">Path</label>
                        <input
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
                      <div className="form-row">
                        <label htmlFor="add-channel-path">Path</label>
                        <input
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
                      <div className="form-row">
                        <label htmlFor="add-channel-video-limit">Video Limit</label>
                        <input
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

            {error && <p className="error">{error.message}</p>}
            <button type="submit" disabled={submitting}>
              {submitting ? 'Creating…' : mode === 'playlist' ? 'Create Playlist' : 'Create Channel'}
            </button>
          </form>

          <Dialog.Close asChild>
            <button className="dialog-close" aria-label="Close">
              ×
            </button>
          </Dialog.Close>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  )
}
