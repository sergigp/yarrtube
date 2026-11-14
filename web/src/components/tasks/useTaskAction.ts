import { useCallback, useState } from 'react'
import { useInvalidateTasks } from '@/api/queries'
import { errorMessage } from '@/lib/errorMessage'

/**
 * Runs `action` on demand, then refreshes the tasks; tracks whether it is in
 * flight and why it last failed, cleared on the next run.
 */
export function useTaskAction(action: () => Promise<void>): {
  run: () => Promise<void>
  working: boolean
  error: string | null
} {
  const invalidateTasks = useInvalidateTasks()
  const [working, setWorking] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const run = useCallback(async () => {
    setWorking(true)
    setError(null)
    try {
      await action()
      await invalidateTasks()
    } catch (err) {
      setError(errorMessage(err))
    } finally {
      setWorking(false)
    }
  }, [action, invalidateTasks])

  return { run, working, error }
}
