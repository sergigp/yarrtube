import { useLocation } from 'react-router-dom'

/**
 * Shows the router's current path and query, so a test can assert where the
 * user ended up (query it as the "Current route" status).
 */
export function RouteProbe() {
  const location = useLocation()
  return <output aria-label="Current route">{`${location.pathname}${location.search}`}</output>
}
