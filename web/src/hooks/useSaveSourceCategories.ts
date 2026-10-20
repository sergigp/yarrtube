import { useCallback } from 'react'
import type { SourceRef } from '@/lib/categories'

/**
 * Returns a function that makes `source` a member of exactly the categories
 * `chosenIds`, one members PUT per changed category, then refetches the
 * categories.
 */
export function useSaveSourceCategories(): (
  source: SourceRef,
  chosenIds: string[],
) => Promise<void> {
  return useCallback(async (_source: SourceRef, _chosenIds: string[]) => {}, [])
}
