/** The message to show for a failed action: an `Error`'s message, or the value itself. */
export function errorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err)
}
