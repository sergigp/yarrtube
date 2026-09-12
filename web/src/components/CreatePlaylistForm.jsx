import { useState } from 'react'
import { createPlaylist } from '../api'

const initialForm = { playlist: '', name: '', path: '', quality: 'high' }

export function CreatePlaylistForm() {
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
      await createPlaylist(form)
      setForm(initialForm)
    } catch (err) {
      setError(err)
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <form className="create-playlist-form" onSubmit={handleSubmit}>
      <h2>Create Playlist</h2>
      <div className="form-row">
        <label htmlFor="create-playlist-id">Playlist ID or URL</label>
        <input
          id="create-playlist-id"
          type="text"
          value={form.playlist}
          onChange={setField('playlist')}
          required
        />
      </div>
      <div className="form-row">
        <label htmlFor="create-playlist-name">Name</label>
        <input
          id="create-playlist-name"
          type="text"
          value={form.name}
          onChange={setField('name')}
          required
        />
      </div>
      <div className="form-row">
        <label htmlFor="create-playlist-path">Path</label>
        <input
          id="create-playlist-path"
          type="text"
          value={form.path}
          onChange={setField('path')}
          required
        />
      </div>
      <div className="form-row">
        <label htmlFor="create-playlist-quality">Quality</label>
        <select
          id="create-playlist-quality"
          value={form.quality}
          onChange={setField('quality')}
        >
          <option value="high">High</option>
          <option value="mid">Mid</option>
          <option value="low">Low</option>
        </select>
      </div>
      {error && <p className="error">{error.message}</p>}
      <button type="submit" disabled={submitting}>
        {submitting ? 'Creating…' : 'Create Playlist'}
      </button>
    </form>
  )
}
