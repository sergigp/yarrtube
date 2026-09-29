import { useDebouncedValue } from './useDebouncedValue'

const LOOKUP_DEBOUNCE_MS = 400

/**
 * Looks `entered` up with `usePreview` once it stops changing, and matches
 * what was found against the `trackedItems` already added, by `sameId`.
 * `addable` holds only once the lookup has caught up with `entered`, found
 * something, and that something isn't tracked yet.
 */
export function useLookup(entered, usePreview, trackedItems, sameId) {
  const debounced = useDebouncedValue(entered, LOOKUP_DEBOUNCE_MS)
  const preview = usePreview(debounced)
  // Until the lookup has caught up with the field, whatever the preview holds
  // belongs to an earlier value.
  const lookingUp = debounced !== entered || preview.isFetching
  const tracked = preview.data && trackedItems?.find((item) => sameId(item.id, preview.data.id))
  const addable = Boolean(preview.data) && !tracked && !lookingUp
  return { preview, lookingUp, tracked, addable }
}
