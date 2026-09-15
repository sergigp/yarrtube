import { useState } from 'react'
import { createChannel } from '../api'

const initialForm = { channel: '', quality: 'high', video_limit: '10' }

export function CreateChannelForm({ onSuccess }) {
  const [form, setForm] = useState(initialForm)
  const [error, setError] = useState(null)
  const [submitting, setSubmitting] = useState(false)

  const setField = (field) => (event) =>
    setForm((prev) => ({ ...prev, [field]: event.target.value }))

  const handleSubmit = async (event) => {
    event.preventDefault()
    setSubmitting(true)
    setError(null)
    try {
      await createChannel({
        channel: form.channel,
        quality: form.quality,
        video_limit: Number(form.video_limit),
      })
      setForm(initialForm)
      onSuccess?.()
    } catch (err) {
      setError(err)
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <form className="create-playlist-form" onSubmit={handleSubmit}>
      <div className="form-row">
        <label htmlFor="create-channel-handle">Channel Handle or URL</label>
        <input
          id="create-channel-handle"
          type="text"
          value={form.channel}
          onChange={setField('channel')}
          placeholder="@somechannel"
          required
        />
      </div>
      <div className="form-row">
        <label htmlFor="create-channel-quality">Quality</label>
        <select
          id="create-channel-quality"
          value={form.quality}
          onChange={setField('quality')}
        >
          <option value="high">High</option>
          <option value="mid">Mid</option>
          <option value="low">Low</option>
        </select>
      </div>
      <div className="form-row">
        <label htmlFor="create-channel-video-limit">Video Limit</label>
        <input
          id="create-channel-video-limit"
          type="number"
          min="1"
          step="1"
          value={form.video_limit}
          onChange={setField('video_limit')}
          required
        />
      </div>
      {error && <p className="error">{error.message}</p>}
      <button type="submit" disabled={submitting}>
        {submitting ? 'Creating…' : 'Create Channel'}
      </button>
    </form>
  )
}
