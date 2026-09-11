import { usePolling } from '../usePolling'
import { fetchPlaylists } from '../api'

export function PlaylistList({ onSelect }) {
  const { data: playlists, error } = usePolling(fetchPlaylists, [])

  if (error) {
    return <p className="error">Failed to load playlists: {error.message}</p>
  }

  if (!playlists) {
    return <p className="muted">Loading playlists…</p>
  }

  if (playlists.length === 0) {
    return <p className="muted">No playlists tracked yet.</p>
  }

  return (
    <ul className="list">
      {playlists.map((playlist) => (
        <li key={playlist.id}>
          <button className="list-item" onClick={() => onSelect(playlist)}>
            <span className="list-item-title">{playlist.name}</span>
            <span className="list-item-meta">
              {playlist.kind} · {playlist.quality}
            </span>
          </button>
        </li>
      ))}
    </ul>
  )
}
