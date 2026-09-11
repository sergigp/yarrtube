import { useEffect, useState } from 'react'

const POLL_INTERVAL_MS = 3000

/**
 * Fetches `fetcher()` immediately, then again every 3s while `active` is
 * true. Re-runs from scratch whenever any value in `deps` changes.
 */
export function usePolling(fetcher, deps, active = true) {
  const [data, setData] = useState(null)
  const [error, setError] = useState(null)

  useEffect(() => {
    if (!active) {
      return undefined
    }

    let cancelled = false

    const load = () => {
      fetcher()
        .then((result) => {
          if (!cancelled) {
            setData(result)
            setError(null)
          }
        })
        .catch((err) => {
          if (!cancelled) {
            setError(err)
          }
        })
    }

    load()
    const interval = setInterval(load, POLL_INTERVAL_MS)

    return () => {
      cancelled = true
      clearInterval(interval)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active, ...deps])

  return { data, error }
}
