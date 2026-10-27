import { appendFileSync, mkdirSync, readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const E2E_DIR = join(dirname(fileURLToPath(import.meta.url)), '..')
const ROUTER_SOURCE = join(E2E_DIR, '..', 'src', 'application', 'http', 'mod.rs')

/** Where the routes the browser hit are appended, one `METHOD /path STATUS` per line. */
export const HIT_ROUTES_FILE = join(E2E_DIR, 'test-results', 'api-routes-hit.txt')

/**
 * Every `METHOD /api/...` the router serves, parsed from its `.route(...)`
 * calls so a new route is expected to be covered without editing this file.
 */
export function routerRoutes() {
  const source = readFileSync(ROUTER_SOURCE, 'utf8')
  const routes = []
  const routeCalls = /\.route\(\s*"([^"]+)",([\s\S]*?)\)\s*(?=\.route\(|\.with_state\()/g
  for (const [, path, handlers] of source.matchAll(routeCalls)) {
    for (const [, method] of handlers.matchAll(/[\s.:(](get|post|patch|put|delete)\(/g)) {
      routes.push(`${method.toUpperCase()} /api${path}`)
    }
  }
  if (routes.length === 0) {
    throw new Error(`no routes parsed from ${ROUTER_SOURCE}; has the router's shape changed?`)
  }
  return routes
}

/** Appends the route `pathname` (a concrete `/api/...` path) matches, with the status it answered. */
export function recordApiRoute(method, pathname, status) {
  mkdirSync(dirname(HIT_ROUTES_FILE), { recursive: true })
  appendFileSync(HIT_ROUTES_FILE, `${method} ${toTemplate(pathname)} ${status}\n`)
}

/**
 * The routes answered with a 2xx at least once so far in this run: a route
 * only ever hit with errors hasn't shown its wiring works.
 */
export function hitRoutes() {
  let lines
  try {
    lines = readFileSync(HIT_ROUTES_FILE, 'utf8').split('\n').filter(Boolean)
  } catch {
    return new Set()
  }
  return new Set(
    lines
      .map((line) => line.match(/^(\S+ \S+) (\d{3})$/))
      .filter((match) => match && match[2].startsWith('2'))
      .map((match) => match[1]),
  )
}

// `/api/playlists/PL123/videos` → `/api/playlists/{id}/videos`, and a
// source's video (`…/videos/abc/subtitles.vtt`) → `…/videos/{video_id}/…`.
// Literal segments the router declares next to a parameter (`preview`,
// `home`) are kept as they are.
function toTemplate(pathname) {
  const segments = pathname.split('/')
  const [, api, resource, id, ...rest] = segments
  if (id === undefined || ['preview', 'home'].includes(id)) {
    return pathname
  }
  const param = resource === 'channels' ? '{handle}' : '{id}'
  const [nested, , ...nestedRest] = rest
  const tail = nested === 'videos' && nestedRest.length > 0 ? [nested, '{video_id}', ...nestedRest] : rest
  return ['', api, resource, param, ...tail].join('/')
}
