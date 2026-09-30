import type { UseQueryResult } from '@tanstack/react-query'
import { useDebouncedValue } from './useDebouncedValue'

const LOOKUP_DEBOUNCE_MS = 400

export interface Lookup<Preview, Tracked> {
  preview: UseQueryResult<Preview, Error>
  /** The lookup hasn't caught up with the field yet. */
  lookingUp: boolean
  /** The already-tracked item matching what was found, if any. */
  tracked: Tracked | undefined
  /** Something was found, isn't tracked yet, and the lookup is current. */
  addable: boolean
}

/**
 * Looks `entered` up with `usePreview` once it stops changing, and matches
 * what was found against the `trackedItems` already added, by `sameId`.
 * `addable` holds only once the lookup has caught up with `entered`, found
 * something, and that something isn't tracked yet.
 */
export function useLookup<Preview extends { id: string }, Tracked extends { id: string }>(
  entered: string,
  usePreview: (value: string) => UseQueryResult<Preview, Error>,
  trackedItems: Tracked[] | undefined,
  sameId: (trackedId: string, previewId: string) => boolean,
): Lookup<Preview, Tracked> {
  const debounced = useDebouncedValue(entered, LOOKUP_DEBOUNCE_MS)
  const preview = usePreview(debounced)
  // Until the lookup has caught up with the field, whatever the preview holds
  // belongs to an earlier value.
  const lookingUp = debounced !== entered || preview.isFetching
  const found = preview.data
  const tracked = found && trackedItems?.find((item) => sameId(item.id, found.id))
  const addable = Boolean(found) && !tracked && !lookingUp
  return { preview, lookingUp, tracked: tracked || undefined, addable }
}
