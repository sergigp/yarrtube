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

export function fetchRecentVideos() {
  return request('/videos/recent')
}

export function fetchTasks() {
  return request('/tasks')
}

export function videoMediaUrl(playlistPath, filename) {
  const encodedPath = playlistPath.split('/').map(encodeURIComponent).join('/')
  const encodedFilename = filename.split('/').map(encodeURIComponent).join('/')
  return `/media/${encodedPath}/${encodedFilename}`
}

export function avatarMediaUrl(filename) {
  return `/avatars/${encodeURIComponent(filename)}`
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

export function fetchChannels() {
  return request('/channels')
}

export function fetchChannelVideos(handle) {
  return request(`/channels/${encodeURIComponent(handle)}/videos`)
}

export async function reconcileChannel(handle) {
  const response = await fetch(`/api/channels/${encodeURIComponent(handle)}/reconcile`, {
    method: 'POST',
  })
  if (!response.ok) {
    const body = await response.json().catch(() => null)
    throw new Error(
      body?.error ?? `request to reconcile channel ${handle} failed with status ${response.status}`,
    )
  }
}

export async function createChannel({ channel, quality, video_limit, path }) {
  const response = await fetch('/api/channels', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ channel, quality, video_limit, path }),
  })
  if (!response.ok) {
    const body = await response.json().catch(() => null)
    throw new Error(
      body?.error ?? `request to /channels failed with status ${response.status}`,
    )
  }
  return response.json()
}

export async function deleteChannel(id) {
  const response = await fetch(`/api/channels/${encodeURIComponent(id)}`, {
    method: 'DELETE',
  })
  if (!response.ok) {
    const body = await response.json().catch(() => null)
    throw new Error(
      body?.error ?? `request to delete channel ${id} failed with status ${response.status}`,
    )
  }
}
