import { useId, useState } from 'react'
import { ChevronDown } from 'lucide-react'
import { avatarMediaUrl, runTasksNow } from '@/api/client'
import { useChannels } from '@/api/queries'
import type { Task } from '@/api/types'
import { NAMED_SOURCES, namedSources, type BatchSource } from '@/lib/tasks'
import { cn } from '@/lib/utils'
import { Button } from '../ui/button'
import { RunNowButton } from './RunNowButton'
import { RunTime } from './RunTime'
import { useTaskAction } from './useTaskAction'
import { TASK_BUTTON_SIZE } from './taskButton'

interface TaskBatchRowProps {
  tasks: Task[]
  sources: BatchSource[]
  runAt: string
  now: Date
}

/** Syncs due close together, shown as one row that expands to list its sources. */
export function TaskBatchRow({ tasks, sources, runAt, now }: TaskBatchRowProps) {
  const [expanded, setExpanded] = useState(false)
  const descriptionId = useId()
  const syncAll = useTaskAction(() => runTasksNow(tasks.map((task) => task.id)))

  return (
    <li className="flex flex-col gap-2 px-3 py-3">
      <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:gap-4">
        <RunTime runAt={runAt} now={now} />
        <span className="flex min-w-0 flex-1 items-center gap-3">
          <SourceAvatars sources={sources.slice(0, NAMED_SOURCES)} />
          <span className="flex min-w-0 flex-col">
            <span id={descriptionId} className="truncate text-sm text-foreground">
              Check {sources.length} channels and playlists for new videos
            </span>
            <span className="truncate text-xs text-muted-foreground">{namedSources(sources)}</span>
          </span>
        </span>
        <span className="flex shrink-0 items-center gap-2">
          <Button
            variant="ghost"
            size="sm"
            className={TASK_BUTTON_SIZE}
            aria-expanded={expanded}
            aria-describedby={descriptionId}
            onClick={() => setExpanded(!expanded)}
          >
            {expanded ? 'Hide sources' : 'Show sources'}
            <ChevronDown className={cn('transition-transform', expanded && 'rotate-180')} aria-hidden />
          </Button>
          <Button
            variant="outline"
            size="sm"
            className={TASK_BUTTON_SIZE}
            disabled={syncAll.working}
            aria-describedby={descriptionId}
            onClick={syncAll.run}
          >
            Sync all now
          </Button>
        </span>
      </div>
      {syncAll.error && (
        <p className="text-xs text-destructive">Sync all now failed: {syncAll.error}</p>
      )}
      {expanded && (
        <ul className="flex flex-col divide-y divide-border rounded-md bg-muted/40 sm:ml-40">
          {sources.map((source) => (
            <SourceRow key={source.taskId} source={source} />
          ))}
        </ul>
      )}
    </li>
  )
}

/** One source of an expanded batch, with its own Sync now. */
function SourceRow({ source }: { source: BatchSource }) {
  const nameId = useId()
  return (
    <li className="flex items-center gap-3 px-3 py-2">
      <span id={nameId} className="min-w-0 flex-1 truncate text-sm text-foreground">
        {source.name}
      </span>
      <span className="text-xs text-muted-foreground">
        {source.kind === 'channel' ? 'Channel' : 'Playlist'}
      </span>
      <RunNowButton
        label="Sync now"
        action={() => runTasksNow([source.taskId])}
        describedBy={nameId}
      />
    </li>
  )
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
