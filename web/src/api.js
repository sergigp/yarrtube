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

/**
 * Lists the immediate subdirectories of `path`, relative to the configured
 * videos root; an empty or omitted `path` lists the root itself. Resolves to
 * `{ root, path, entries }`, where `root` is the absolute videos root the
 * add dialog's destination preview is built from.
 */
export function fetchDirectories(path) {
  const query = path ? `?path=${encodeURIComponent(path)}` : ''
  return request(`/directories${query}`)
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

export async function markChannelWatched(handle) {
  const response = await fetch(`/api/channels/${encodeURIComponent(handle)}/watched`, {
    method: 'POST',
  })
  if (!response.ok) {
    const body = await response.json().catch(() => null)
    throw new Error(
      body?.error ?? `request to mark channel ${handle} watched failed with status ${response.status}`,
    )
  }
}

export async function recordVideoProgress(youtubeId, { position_seconds, duration_seconds }) {
  const response = await fetch(`/api/videos/${encodeURIComponent(youtubeId)}/progress`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ position_seconds, duration_seconds }),
  })
  if (!response.ok) {
    const body = await response.json().catch(() => null)
    throw new Error(
      body?.error ?? `request to record progress of video ${youtubeId} failed with status ${response.status}`,
    )
  }
}

/**
 * Records progress with `navigator.sendBeacon`, which the browser still
 * delivers while the page is being closed or hidden.
 */
export function beaconVideoProgress(youtubeId, { position_seconds, duration_seconds }) {
  const body = new Blob([JSON.stringify({ position_seconds, duration_seconds })], {
    type: 'application/json',
  })
  navigator.sendBeacon(`/api/videos/${encodeURIComponent(youtubeId)}/progress`, body)
}
