import { expect } from '@playwright/test'
import { existsSync, readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'

// The host directory mounted as the container's videos root, so specs can
// check what actually landed on disk rather than only what the UI reports.
const VIDEOS_DIR = process.env.SMOKE_VIDEOS_DIR

const MEDIA_EXTENSIONS = ['mp4', 'mkv', 'webm']

/** The host path of `relative` (a path under the videos root). */
export function libraryPath(relative) {
  return join(VIDEOS_DIR, relative)
}

/**
 * Asserts the show folder at `showPath` (relative to the videos root) holds
 * `videoTitle` as a TV-show episode: a `tvshow.nfo` naming `showTitle`, a
 * season folder, and an episode's media, `.nfo` and `.jpg` sharing one
 * `S<season>E<episode> - …` stem whose numbers the `.nfo` agrees with.
 * Playlists use `Season 1`/`S01E<nnn>`, channels `Season <year>`/
 * `S<year>E<MMDDii>`. Channels also get a `poster.jpg` from their avatar.
 * Retried for a while, since the thumbnail is fetched after the download.
 */
export async function expectEpisodeOnDisk({ showPath, kind, showTitle, videoTitle }) {
  const showDir = libraryPath(showPath)
  await expect(async () => {
    const show = readFileSync(join(showDir, 'tvshow.nfo'), 'utf8')
    expect(xmlText(show, 'title')).toBe(showTitle)
    expect(xmlText(show, 'uniqueid')).not.toBe('')
    if (kind === 'channel') {
      expect(existsSync(join(showDir, 'poster.jpg')), 'poster.jpg').toBe(true)
    }

    const episode = findEpisode(showDir, videoTitle)
    const { season, episodeCode } = episode
    if (kind === 'playlist') {
      expect(episode.seasonDir).toBe('Season 1')
      expect(season).toBe('01')
      expect(episodeCode).toMatch(/^\d{3}$/)
    } else {
      expect(episode.seasonDir).toBe(`Season ${season}`)
      expect(season).toMatch(/^\d{4}$/)
      expect(episodeCode).toMatch(/^\d{6}$/)
    }
    expect(Number(xmlText(episode.nfo, 'season'))).toBe(Number(season))
    expect(Number(xmlText(episode.nfo, 'episode'))).toBe(Number(episodeCode))
    expect(xmlText(episode.nfo, 'uniqueid')).toMatch(/^[\w-]{11}$/)
    expect(episode.siblings, `media next to ${episode.stem}.nfo`).toEqual(
      expect.arrayContaining([expect.stringMatching(new RegExp(`\\.(${MEDIA_EXTENSIONS.join('|')})$`))]),
    )
    expect(episode.siblings, `thumbnail next to ${episode.stem}.nfo`).toContain(`${episode.stem}.jpg`)
  }).toPass({ timeout: 30_000 })
}

/** Asserts the folder at `relative` is removed (deletion runs as a task). */
export async function expectGoneFromDisk(relative) {
  await expect.poll(() => existsSync(libraryPath(relative)), { timeout: 30_000 }).toBe(false)
}

/** Finds the episode whose `.nfo` title is `videoTitle`, in any season folder. */
function findEpisode(showDir, videoTitle) {
  const seasonDirs = readdirSync(showDir).filter((name) => /^Season \d+$/.test(name))
  for (const seasonDir of seasonDirs) {
    const files = readdirSync(join(showDir, seasonDir))
    for (const file of files.filter((name) => name.endsWith('.nfo'))) {
      const nfo = readFileSync(join(showDir, seasonDir, file), 'utf8')
      if (xmlText(nfo, 'title') !== videoTitle) {
        continue
      }
      const stem = file.slice(0, -'.nfo'.length)
      const match = stem.match(/^S(\d+)E(\d+) - .+$/)
      expect(match, `episode file name "${file}"`).not.toBeNull()
      return {
        seasonDir,
        stem,
        nfo,
        season: match[1],
        episodeCode: match[2],
        siblings: files.filter((name) => name.startsWith(`${stem}.`)),
      }
    }
  }
  throw new Error(`no episode .nfo titled "${videoTitle}" under ${showDir} (seasons: ${seasonDirs.join(', ')})`)
}

function xmlText(xml, tag) {
  const match = xml.match(new RegExp(`<${tag}(?:\\s[^>]*)?>([^<]*)</${tag}>`))
  expect(match, `<${tag}> in ${xml.slice(0, 200)}`).not.toBeNull()
  return match[1]
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&apos;/g, "'")
    .replace(/&amp;/g, '&')
}
