import { useVersion } from '@/api/queries'
import type { BuildCommit } from '@/api/types'
import { badgeVariants } from '@/components/ui/badge'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { formatDateTime } from '@/lib/formatDateTime'

const COMMIT_URL = 'https://github.com/sergigp/yarrtube/commit/'

/**
 * The running build, next to the logo: the version as muted text, or for an
 * unreleased `main` build an `edge` badge that opens its commit. Renders
 * nothing while loading or on any fetch error.
 */
export function VersionLabel() {
  const { data } = useVersion()
  if (!data) {
    return null
  }

  if (data.channel === 'edge' && data.commit) {
    return <EdgeBadge commit={data.commit} />
  }

  return <span className="text-xs text-muted-foreground">{data.version}</span>
}

function EdgeBadge({ commit }: { commit: BuildCommit }) {
  return (
    <Popover>
      <PopoverTrigger className={badgeVariants({ variant: 'secondary' })}>edge</PopoverTrigger>
      <PopoverContent align="start" className="flex flex-col gap-1 text-sm">
        <a
          href={`${COMMIT_URL}${commit.sha}`}
          target="_blank"
          rel="noreferrer"
          className="font-mono text-primary underline-offset-4 hover:underline"
        >
          {commit.sha.slice(0, 7)}
        </a>
        <p>{commit.subject}</p>
        <p className="text-xs text-muted-foreground">{formatDateTime(commit.date)}</p>
      </PopoverContent>
    </Popover>
  )
}
