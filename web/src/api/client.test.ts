import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  avatarMediaUrl,
  beaconVideoProgress,
  createChannel,
  createPlaylist,
  deletePlaylist,
  fetchDirectories,
  fetchPlaylists,
  recordVideoProgress,
  videoMediaUrl,
} from './client'
import { mockApi } from '@/test/helpers'

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('media URLs', () => {
  it('encodes each path segment of a video URL separately', () => {
    expect(videoMediaUrl('playlists/kids songs', 'a video #1.mp4')).toBe(
      '/media/playlists/kids%20songs/a%20video%20%231.mp4',
    )
  })

  it('encodes avatar filenames', () => {
    expect(avatarMediaUrl('avatar 1.jpg')).toBe('/avatars/avatar%201.jpg')
  })
})

describe('GET requests', () => {
  it('resolves with the parsed JSON body', async () => {
    mockApi({ 'GET /api/playlists': [{ id: 'PL1' }] })

    await expect(fetchPlaylists()).resolves.toEqual([{ id: 'PL1' }])
  })

  it('encodes the directory path into the query string', async () => {
    const fetchMock = mockApi({
      'GET /api/directories?path=kids%2Fsongs': { root: '/videos', path: 'kids/songs', entries: [] },
    })

    await fetchDirectories('kids/songs')

    expect(fetchMock).toHaveBeenCalledOnce()
  })

  it('requests the root listing without a query string', async () => {
    mockApi({ 'GET /api/directories': { root: '/videos', path: '', entries: [] } })

    await expect(fetchDirectories()).resolves.toEqual({ root: '/videos', path: '', entries: [] })
  })

  it('rejects with the server-provided error message', async () => {
    mockApi({ 'GET /api/playlists': { status: 502, error: 'youtube is down' } })

    await expect(fetchPlaylists()).rejects.toThrow('youtube is down')
  })

  it('falls back to a generic message when the error body is not JSON', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response('gateway timeout', { status: 504 })),
    )

    await expect(fetchPlaylists()).rejects.toThrow(
      'request to /playlists failed with status 504',
    )
  })
})

describe('mutations', () => {
  it('POSTs the playlist creation payload as JSON', async () => {
    const fetchMock = mockApi({ 'POST /api/playlists': { id: 'PL1' } })

    await createPlaylist({ playlist: 'PL1', path: 'playlists/mix', quality: 'high' })

    const [, init] = fetchMock.mock.calls[0] as [string, RequestInit]
    expect(init.headers).toEqual({ 'content-type': 'application/json' })
    expect(JSON.parse(init.body as string)).toEqual({
      playlist: 'PL1',
      path: 'playlists/mix',
      quality: 'high',
    })
  })

  it('POSTs the channel creation payload as JSON', async () => {
    const fetchMock = mockApi({ 'POST /api/channels': { id: 'chan' } })

    await createChannel({ channel: '@chan', quality: 'mid', video_limit: 3, path: 'channels/chan' })

    const [, init] = fetchMock.mock.calls[0] as [string, RequestInit]
    expect(JSON.parse(init.body as string)).toEqual({
      channel: '@chan',
      quality: 'mid',
      video_limit: 3,
      path: 'channels/chan',
    })
  })

  it('deletes without expecting a response body', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response(null, { status: 204 })),
    )

    await expect(deletePlaylist('PL1')).resolves.toBeUndefined()
  })

  it('names the action in a delete failure without a server message', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response(null, { status: 500 })),
    )

    await expect(deletePlaylist('PL1')).rejects.toThrow(
      'request to delete playlist PL1 failed with status 500',
    )
  })

  it('records progress and resolves with the watched flag', async () => {
    const fetchMock = mockApi({ 'POST /api/videos/abc/progress': { watched: true } })

    await expect(
      recordVideoProgress('abc', { position_seconds: 42, duration_seconds: 100 }),
    ).resolves.toEqual({ watched: true })
    const [, init] = fetchMock.mock.calls[0] as [string, RequestInit]
    expect(JSON.parse(init.body as string)).toEqual({
      position_seconds: 42,
      duration_seconds: 100,
    })
  })

  it('sends beacon progress through navigator.sendBeacon', () => {
    const sendBeacon = vi.fn(() => true)
    vi.stubGlobal('navigator', { sendBeacon })

    beaconVideoProgress('a b', { position_seconds: 10, duration_seconds: 60 })

    expect(sendBeacon).toHaveBeenCalledWith('/api/videos/a%20b/progress', expect.any(Blob))
  })
})
