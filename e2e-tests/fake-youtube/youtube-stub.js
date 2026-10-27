// The fake YouTube Data API the end-to-end suite points the daemon at
// (YARRTUBE_YOUTUBE_API_URL), serving catalogue.json in the shape each of
// the daemon's Data API adapters reads (see their contract tests for what
// the real API returns). Also serves the fixture images: video thumbnails
// and the channel's avatar. Started by scripts/run-tests.sh on
// E2E_YOUTUBE_STUB_PORT.
import { createServer } from 'node:http'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const PORT = Number(process.env.E2E_YOUTUBE_STUB_PORT ?? 18326)
const HERE = dirname(fileURLToPath(import.meta.url))
const CATALOGUE = JSON.parse(readFileSync(join(HERE, 'catalogue.json'), 'utf8'))
const SAMPLES_DIR = join(HERE, '..', 'samples')
// How the daemon's container reaches this stub; the e2e fixtures map it to
// localhost for the browser.
const IMAGES_URL = `http://host.docker.internal:${PORT}/images`

const videos = new Map(CATALOGUE.videos.map((video) => [video.id, video]))

const ROUTES = {
  '/youtube/v3/playlists': (query) => {
    const playlist = CATALOGUE.playlists.find((candidate) => candidate.id === query.get('id'))
    return {
      items: playlist
        ? [{ snippet: { title: playlist.title }, contentDetails: { itemCount: playlist.videos.length } }]
        : [],
    }
  },
  // Every fixture playlist fits in one page.
  '/youtube/v3/playlistItems': (query) => {
    const playlist = CATALOGUE.playlists.find((candidate) => candidate.id === query.get('playlistId'))
    return {
      items: (playlist?.videos ?? []).map((id, position) => ({
        snippet: { title: videos.get(id).title, position, resourceId: { videoId: id } },
        status: { privacyStatus: 'public' },
      })),
    }
  },
  '/youtube/v3/videos': (query) => ({
    items: (query.get('id') ?? '')
      .split(',')
      .filter((id) => videos.has(id))
      .map((id) => {
        const video = videos.get(id)
        return {
          id,
          snippet: {
            title: video.title,
            description: video.description,
            channelTitle: channelOf(id)?.title ?? 'Yarrtube Fixtures',
            publishedAt: video.publishedAt,
            tags: video.tags,
            categoryId: video.categoryId,
            liveBroadcastContent: video.liveBroadcastContent,
            thumbnails: { default: { url: `${IMAGES_URL}/${CATALOGUE.sample}.jpg` } },
          },
        }
      }),
  }),
  '/youtube/v3/channels': (query) => {
    const handle = (query.get('forHandle') ?? '').toLowerCase()
    const channel = CATALOGUE.channels.find((candidate) => candidate.handle.toLowerCase() === handle)
    return {
      items: channel
        ? [
            {
              id: channel.id,
              snippet: {
                title: channel.title,
                thumbnails: { default: { url: `${IMAGES_URL}/${CATALOGUE.sample}.jpg` } },
              },
            },
          ]
        : [],
    }
  },
}

function channelOf(videoId) {
  return CATALOGUE.channels.find((channel) => channel.videos.includes(videoId))
}

createServer((request, response) => {
  const url = new URL(request.url, `http://localhost:${PORT}`)
  const route = ROUTES[url.pathname]
  if (route) {
    response.writeHead(200, { 'content-type': 'application/json' }).end(JSON.stringify(route(url.searchParams)))
    return
  }
  const image = url.pathname.match(/^\/images\/([\w.-]+\.jpg)$/)
  if (image) {
    try {
      const body = readFileSync(join(SAMPLES_DIR, image[1]))
      response.writeHead(200, { 'content-type': 'image/jpeg', 'access-control-allow-origin': '*' }).end(body)
    } catch {
      response.writeHead(404).end()
    }
    return
  }
  response.writeHead(404).end()
}).listen(PORT, () => console.log(`YouTube stub listening on ${PORT}`))
