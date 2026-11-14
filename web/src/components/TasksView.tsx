import { useAiEnabled, useTasks } from '@/api/queries'
import type { Task } from '@/api/types'
import { buildTaskList, statusLine, type TaskEntry, type TaskSectionId } from '@/lib/tasks'
import { VIEW_WIDTH } from '@/lib/viewWidth'
import { Beacon } from './Beacon'
import { TaskBatchRow } from './tasks/TaskBatchRow'
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

  if (error) {
    return <p className="text-sm text-destructive">Failed to load tasks: {error.message}</p>
  }

  if (!tasks) {
    return <p className="text-sm text-muted-foreground">Loading tasks…</p>
  }

  const now = new Date()
  const { sections } = buildTaskList(tasks, { filter: 'all', query: '', aiEnabled, now })

  return (
    <div className="flex flex-col gap-6">
      <StatusLine tasks={tasks} now={now} />
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
              <Entry key={entryKey(entry)} entry={entry} now={now} />
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

function Entry({ entry, now }: { entry: TaskEntry; now: Date }) {
  return entry.kind === 'batch' ? (
    <TaskBatchRow tasks={entry.tasks} sources={entry.sources} runAt={entry.runAt} now={now} />
  ) : (
    <TaskRow task={entry.task} now={now} />
  )
}

function entryKey(entry: TaskEntry): string {
  return entry.kind === 'batch' ? `batch-${entry.tasks[0].id}` : `task-${entry.task.id}`
}
