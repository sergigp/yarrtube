import type { Task } from '@/api/types'

interface AttentionCardProps {
  task: Task
  now: Date
}

/** A failing task, its reason in plain words, with Retry now and Stop retrying. */
export function AttentionCard(_props: AttentionCardProps) {
  return null
}
