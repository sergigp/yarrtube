/** The message to show for a failed action: an `Error`'s message, or the value itself. */
export function errorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err)
}

/** What to show when a save failed with `error`. */
export function couldntSave(error: string): string {
  return `Couldn't save: ${error}. Try again.`
}
