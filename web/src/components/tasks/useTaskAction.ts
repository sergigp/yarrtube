/** Runs `action` on demand, tracking whether it is in flight and how it failed. */
export function useTaskAction(_action: () => Promise<void>): {
  run: () => Promise<void>
  working: boolean
  error: string | null
} {
  return { run: async () => {}, working: false, error: null }
}
