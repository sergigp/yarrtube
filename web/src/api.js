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

export async function deletePlaylist(id) {
  const response = await fetch(`/api/playlists/${encodeURIComponent(id)}`, {
    method: 'DELETE',
  })
  if (!response.ok) {
    const body = await response.json().catch(() => null)
    throw new Error(
      body?.error ?? `request to delete playlist ${id} failed with status ${response.status}`,
    )
  }
}

export async function reconcilePlaylist(id) {
  const response = await fetch(`/api/playlists/${encodeURIComponent(id)}/reconcile`, {
    method: 'POST',
  })
  if (!response.ok) {
    const body = await response.json().catch(() => null)
    throw new Error(
      body?.error ?? `request to reconcile playlist ${id} failed with status ${response.status}`,
    )
  }
}

export async function deleteVideoFromCustomPlaylist(playlistId, videoId) {
  const response = await fetch(
    `/api/custom-playlists/${encodeURIComponent(playlistId)}/videos/${encodeURIComponent(videoId)}`,
    { method: 'DELETE' },
  )
  if (!response.ok) {
    const body = await response.json().catch(() => null)
    throw new Error(
      body?.error ?? `request to delete video ${videoId} failed with status ${response.status}`,
    )
  }
}
