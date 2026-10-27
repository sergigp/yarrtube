import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

// The fake YouTube's world (see fake-youtube/catalogue.json), for specs to
// use by name instead of hardcoding ids and titles.
const CATALOGUE = JSON.parse(
  readFileSync(join(dirname(fileURLToPath(import.meta.url)), '..', 'fake-youtube', 'catalogue.json'), 'utf8'),
)

function playlist(id) {
  return CATALOGUE.playlists.find((candidate) => candidate.id === id)
}

/** The golden-path playlist, holding one regular video. */
export const PLAYLIST = playlist('PLyarrtubeFixtureTest')
/** Its video downloads for about 20 seconds. */
export const SLOW_PLAYLIST = playlist('PLyarrtubeFixtureSlow')
/** Its video's download always fails. */
export const BROKEN_PLAYLIST = playlist('PLyarrtubeFixtureBroken')
/** An upcoming premiere, then a regular video. */
export const PREMIERE_PLAYLIST = playlist('PLyarrtubeFixturePremiere')
/** 16 empty playlists, enough for the search boxes to show. */
export const SEARCH_PLAYLISTS = CATALOGUE.playlists.filter((candidate) => candidate.id.startsWith('PLyarrtubeFixtureSearch'))
/** Two videos, newest first. */
export const CHANNEL = CATALOGUE.channels[0]
