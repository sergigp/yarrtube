import { useVersion } from '@/api/queries'

/**
 * The running version, as muted text next to the logo. Renders nothing while
 * loading or on any fetch error.
 */
export function VersionLabel() {
  const { data } = useVersion()
  if (!data) {
    return null
  }

  return <span className="text-xs text-muted-foreground">{data.version}</span>
}
