import { appendFileSync, mkdirSync, readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const SMOKE_DIR = join(dirname(fileURLToPath(import.meta.url)), '..')
const ROUTER_SOURCE = join(SMOKE_DIR, '..', 'src', 'application', 'http', 'mod.rs')

/** Where the routes the browser hit are appended, one `METHOD /path` per line. */
export const HIT_ROUTES_FILE = join(SMOKE_DIR, 'test-results', 'api-routes-hit.txt')

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

/** Appends the route `pathname` (a concrete `/api/...` path) matches. */
export function recordApiRoute(method, pathname) {
  mkdirSync(dirname(HIT_ROUTES_FILE), { recursive: true })
  appendFileSync(HIT_ROUTES_FILE, `${method} ${toTemplate(pathname)}\n`)
}

/** The routes recorded so far in this run. */
export function hitRoutes() {
  try {
    return new Set(readFileSync(HIT_ROUTES_FILE, 'utf8').split('\n').filter(Boolean))
  } catch {
    return new Set()
  }
}

// `/api/playlists/PL123/videos` → `/api/playlists/{id}/videos`. Literal
// segments the router declares next to a parameter (`preview`, `home`) are
// kept as they are.
function toTemplate(pathname) {
  const segments = pathname.split('/')
  const [, api, resource, id, ...rest] = segments
  if (id === undefined || ['preview', 'home'].includes(id)) {
    return pathname
  }
  const param = resource === 'channels' ? '{handle}' : '{id}'
  return ['', api, resource, param, ...rest].join('/')
}
