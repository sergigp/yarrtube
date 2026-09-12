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
