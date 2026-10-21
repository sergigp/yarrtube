/** Whether any listed playlist or channel still has work in progress. */
export function anyWorkInProgress(items: { work_in_progress: boolean }[] | undefined): boolean {
  return items?.some((item) => item.work_in_progress) ?? false
}
