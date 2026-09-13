import { usePolling } from '../usePolling'
import { fetchPlaylists } from '../api'
import { PlaylistActionsMenu } from './PlaylistActionsMenu'

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
        <li key={playlist.id} className="list-row">
          <button className="list-item" onClick={() => onSelect(playlist)}>
            <span className="list-item-title">{playlist.name}</span>
            <span className="chip-group">
              <span className="status-badge">{playlist.kind}</span>
              <span className="status-badge">{playlist.quality}</span>
            </span>
          </button>
          <PlaylistActionsMenu playlist={playlist} />
        </li>
      ))}
    </ul>
  )
}
