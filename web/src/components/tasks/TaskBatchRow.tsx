import type { Task } from '@/api/types'
import type { BatchSource } from '@/lib/tasks'

interface TaskBatchRowProps {
  tasks: Task[]
  sources: BatchSource[]
  runAt: string
  now: Date
}

/** Syncs due close together, shown as one expandable row. */
export function TaskBatchRow(_props: TaskBatchRowProps) {
  return null
}
