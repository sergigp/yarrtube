async function request(path) {
  const response = await fetch(`/api${path}`)
  if (!response.ok) {
    throw new Error(`request to ${path} failed with status ${response.status}`)
  }
  return response.json()
}

export function fetchPlaylists() {
  return request('/playlists')
}

export function fetchVideos(playlistId) {
  return request(`/playlists/${encodeURIComponent(playlistId)}/videos`)
}

export function fetchTasks() {
  return request('/tasks')
}

export function videoMediaUrl(playlistPath, filename) {
  return `/media/${playlistPath}/${filename}`
}

export async function createPlaylist({ playlist, name, path, quality }) {
  const response = await fetch('/api/playlists', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ playlist, name, path, quality }),
  })
  if (!response.ok) {
    const body = await response.json().catch(() => null)
    throw new Error(
      body?.error ?? `request to /playlists failed with status ${response.status}`,
    )
  }
  return response.json()
}
