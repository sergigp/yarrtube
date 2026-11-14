import { useState } from 'react'
import { ChevronDown } from 'lucide-react'
import { avatarMediaUrl, runTasksNow } from '@/api/client'
import { useChannels } from '@/api/queries'
import type { Task } from '@/api/types'
import type { BatchSource } from '@/lib/tasks'
import { formatRunTime } from '@/lib/taskTimes'
import { cn } from '@/lib/utils'
import { Button } from '../ui/button'
import { RunNowButton } from './RunNowButton'
import { useTaskAction } from './useTaskAction'

/** How many sources a collapsed batch names before "and N more". */
const NAMED_SOURCES = 3

interface TaskBatchRowProps {
  tasks: Task[]
  sources: BatchSource[]
  runAt: string
  now: Date
}

/** Syncs due close together, shown as one row that expands to list its sources. */
export function TaskBatchRow({ tasks, sources, runAt, now }: TaskBatchRowProps) {
  const [expanded, setExpanded] = useState(false)
  const syncAll = useTaskAction(() => runTasksNow(tasks.map((task) => task.id)))
  const { clock, relative } = formatRunTime(runAt, now)

  return (
    <li className="flex flex-col gap-2 px-3 py-3">
      <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:gap-4">
        <span className="flex shrink-0 items-baseline gap-1.5 text-xs text-muted-foreground tabular-nums sm:w-36">
          <span className="font-medium text-foreground">{clock}</span>
          <span aria-hidden>·</span>
          <span>{relative}</span>
        </span>
        <span className="flex min-w-0 flex-1 items-center gap-3">
          <SourceAvatars sources={sources.slice(0, NAMED_SOURCES)} />
          <span className="flex min-w-0 flex-col">
            <span className="truncate text-sm text-foreground">
              Check {sources.length} channels and playlists for new videos
            </span>
            <span className="truncate text-xs text-muted-foreground">{namedSources(sources)}</span>
          </span>
        </span>
        <span className="flex shrink-0 items-center gap-2">
          <Button
            variant="ghost"
            size="sm"
            className="min-h-11 sm:min-h-7"
            aria-expanded={expanded}
            onClick={() => setExpanded(!expanded)}
          >
            {expanded ? 'Hide sources' : 'Show sources'}
            <ChevronDown className={cn('transition-transform', expanded && 'rotate-180')} aria-hidden />
          </Button>
          <Button
            variant="outline"
            size="sm"
            className="min-h-11 sm:min-h-7"
            disabled={syncAll.working}
            onClick={syncAll.run}
          >
            Sync all now
          </Button>
        </span>
      </div>
      {syncAll.error && (
        <p className="text-xs text-destructive">Sync failed: {syncAll.error}</p>
      )}
      {expanded && (
        <ul className="flex flex-col divide-y divide-border rounded-md bg-muted/40 sm:ml-40">
          {sources.map((source) => (
            <li key={source.taskId} className="flex items-center gap-3 px-3 py-2">
              <span className="min-w-0 flex-1 truncate text-sm text-foreground">{source.name}</span>
              <span className="text-xs text-muted-foreground">
                {source.kind === 'channel' ? 'Channel' : 'Playlist'}
              </span>
              <RunNowButton label="Sync now" action={() => runTasksNow([source.taskId])} />
            </li>
          ))}
        </ul>
      )}
    </li>
  )
}

/** The first sources' names, then how many more: "A, B, C and 13 more". */
function namedSources(sources: BatchSource[]): string {
  const names = sources.slice(0, NAMED_SOURCES).map((source) => source.name)
  const more = sources.length - names.length
  if (more > 0) {
    return `${names.join(', ')} and ${more} more`
  }
  return names.length > 1 ? `${names.slice(0, -1).join(', ')} and ${names.at(-1)}` : names.join('')
}

/** Overlapping avatars of the sources: a channel's picture, or an initial. */
function SourceAvatars({ sources }: { sources: BatchSource[] }) {
  const { data: channels } = useChannels()
  const avatars = new Map(channels?.map((channel) => [channel.id, channel.avatar_filename]))
  return (
    <span className="hidden shrink-0 -space-x-2 sm:flex" aria-hidden>
      {sources.map((source) => {
        const filename = source.kind === 'channel' ? avatars.get(source.sourceId) : null
        return filename ? (
          <img
            key={source.taskId}
            src={avatarMediaUrl(filename)}
            alt=""
            className="size-6 rounded-full border-2 border-card object-cover"
          />
        ) : (
          <span
            key={source.taskId}
            className="flex size-6 items-center justify-center rounded-full border-2 border-card bg-muted text-[10px] font-semibold text-muted-foreground uppercase"
          >
            {source.name.charAt(0)}
          </span>
        )
      })}
    </span>
  )
}
