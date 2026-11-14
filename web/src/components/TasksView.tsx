import { useState } from 'react'
import { useAiEnabled, useTasks } from '@/api/queries'
import type { Task } from '@/api/types'
import {
  buildTaskList,
  statusLine,
  type TaskEntry,
  type TaskFilter,
  type TaskSectionId,
} from '@/lib/tasks'
import { VIEW_WIDTH } from '@/lib/viewWidth'
import { Beacon } from './Beacon'
import { Input } from './ui/input'
import { AttentionCard } from './tasks/AttentionCard'
import { TaskBatchRow } from './tasks/TaskBatchRow'
import { TaskFilterChips } from './tasks/TaskFilterChips'
import { TaskRow } from './tasks/TaskRow'

const SECTION_TITLES: Record<TaskSectionId, string> = {
  running: 'Running now',
  attention: 'Needs attention',
  nextHour: 'Next hour',
  later: 'Later',
}

/**
 * The tasks view: a "Tasks" heading, a sentence saying what is happening now,
 * and every task in sections, at a readable width and aligned to the left
 * like the other views.
 */
export function TasksView() {
  return (
    <div className={VIEW_WIDTH}>
      <h2 className="mb-1 truncate font-heading text-lg font-semibold text-foreground md:text-xl">
        Tasks
      </h2>
      <TaskList />
    </div>
  )
}

function TaskList() {
  const { data: tasks, error } = useTasks()
  const aiEnabled = useAiEnabled()
  const [filter, setFilter] = useState<TaskFilter>('all')
  const [query, setQuery] = useState('')

  if (error) {
    return <p className="text-sm text-destructive">Failed to load tasks: {error.message}</p>
  }

  if (!tasks) {
    return <p className="text-sm text-muted-foreground">Loading tasks…</p>
  }

  if (tasks.length === 0) {
    return <p className="text-sm text-muted-foreground">All quiet. Nothing is scheduled.</p>
  }

  const now = new Date()
  const { chips, sections } = buildTaskList(tasks, { filter, query, aiEnabled, now })

  return (
    <div className="flex flex-col gap-6">
      <StatusLine tasks={tasks} now={now} />
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <TaskFilterChips chips={chips} selected={filter} onSelect={setFilter} />
        <Input
          type="search"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="Search"
          aria-label="Search tasks"
          className="min-h-11 sm:min-h-8 sm:w-56"
        />
      </div>
      {sections.length === 0 && (
        <p className="text-sm text-muted-foreground">Nothing matches your search.</p>
      )}
      {sections.map((section) => (
        <section key={section.id} aria-labelledby={`tasks-${section.id}`}>
          <h3
            id={`tasks-${section.id}`}
            className="mb-2 text-xs font-semibold tracking-wide text-muted-foreground uppercase"
          >
            {SECTION_TITLES[section.id]}
          </h3>
          <ul className="flex flex-col divide-y divide-border rounded-lg border border-border">
            {section.entries.map((entry) => (
              <Entry key={entryKey(entry)} entry={entry} section={section.id} now={now} />
            ))}
          </ul>
        </section>
      ))}
    </div>
  )
}

function StatusLine({ tasks, now }: { tasks: Task[]; now: Date }) {
  const running = tasks.some((task) => task.status === 'running')
  return (
    <p className="flex items-center gap-2 text-sm text-muted-foreground">
      {running && <Beacon variant="live" label="Tasks running" />}
      <span>{statusLine(tasks, now)}</span>
    </p>
  )
}

interface EntryProps {
  entry: TaskEntry
  section: TaskSectionId
  now: Date
}

function Entry({ entry, section, now }: EntryProps) {
  if (entry.kind === 'batch') {
    return <TaskBatchRow tasks={entry.tasks} sources={entry.sources} runAt={entry.runAt} now={now} />
  }
  return section === 'attention' ? (
    <AttentionCard task={entry.task} now={now} />
  ) : (
    <TaskRow task={entry.task} now={now} />
  )
}

function entryKey(entry: TaskEntry): string {
  return entry.kind === 'batch' ? `batch-${entry.tasks[0].id}` : `task-${entry.task.id}`
}
