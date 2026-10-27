import { hitRoutes, routerRoutes } from './helpers/routeCoverage.js'

/**
 * Fails a passing full run when the browser never got a 2xx from one of the
 * router's `/api` routes: Rust handler tests bypass the router, so this suite is
 * the only coverage of its wiring. Enabled by `E2E_CHECK_ROUTE_COVERAGE`
 * (set by `scripts/run-e2e-tests.sh`), since a filtered run legitimately
 * misses routes.
 */
export default class RouteCoverageReporter {
  onEnd(result) {
    if (!process.env.E2E_CHECK_ROUTE_COVERAGE || result.status !== 'passed') {
      return
    }
    const hit = hitRoutes()
    const missed = routerRoutes().filter((route) => !hit.has(route))
    if (missed.length === 0) {
      console.log('All /api routes were exercised by the suite.')
      return
    }
    console.error(`/api routes no spec got a 2xx from through the UI:\n  ${missed.join('\n  ')}`)
    return { status: 'failed' }
  }

  printsToStdio() {
    return false
  }
}
