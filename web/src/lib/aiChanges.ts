import type { PreferencesPatch } from '@/api/types'

/** Human names of the changed settings, deduped, in page order. */
export function changedSettingNames(_patch: PreferencesPatch): string[] {
  return []
}

/** "1 unsaved change" or "<n> unsaved changes". */
export function unsavedChangesLabel(_count: number): string {
  return ''
}
