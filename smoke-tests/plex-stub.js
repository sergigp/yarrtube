// A stand-in Plex server for the smoke suite: answers the two requests
// yarrtube makes (list sections, refresh a path in one) and records every
// request so specs can check Plex was asked to scan what it should.
//
// Sections: 1 holds the playlists, 2 the channels, 3 an unrelated folder,
// all under `/plex/yarrtube`, the `YARRTUBE_PLEX_VIDEOS_PATH` the container
// is started with.
//
// Control endpoints for specs: `GET /__requests` (recorded requests),
// `POST /__reset` (clears them).
import { createServer } from 'node:http'

const PORT = Number(process.env.SMOKE_PLEX_STUB_PORT ?? 18324)

const SECTIONS = {
  MediaContainer: {
    Directory: [
      { key: '1', title: 'Playlists', Location: [{ id: 1, path: '/plex/yarrtube/playlists' }] },
      { key: '2', title: 'Channels', Location: [{ id: 2, path: '/plex/yarrtube/channels' }] },
      { key: '3', title: 'Elsewhere', Location: [{ id: 3, path: '/elsewhere' }] },
    ],
  },
}

let requests = []

createServer((req, res) => {
  const url = new URL(req.url, 'http://stub')
  if (url.pathname === '/__requests') {
    return json(res, requests)
  }
  if (url.pathname === '/__reset' && req.method === 'POST') {
    requests = []
    return json(res, {})
  }
  requests.push({
    method: req.method,
    path: url.pathname,
    query: Object.fromEntries(url.searchParams),
    token: req.headers['x-plex-token'] ?? null,
  })
  if (req.method === 'GET' && url.pathname === '/library/sections') {
    return json(res, SECTIONS)
  }
  if (req.method === 'GET' && /^\/library\/sections\/\d+\/refresh$/.test(url.pathname)) {
    return json(res, {})
  }
  res.writeHead(404).end()
}).listen(PORT, '0.0.0.0', () => console.log(`plex stub listening on ${PORT}`))

function json(res, body) {
  res.writeHead(200, { 'Content-Type': 'application/json' }).end(JSON.stringify(body))
}
