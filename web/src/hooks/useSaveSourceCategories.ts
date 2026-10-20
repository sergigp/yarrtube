import { useCallback } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { listCategories, setCategoryMembers } from '@/api/client'
import { queryKeys } from '@/api/queries'
import { membershipChanges, type SourceRef } from '@/lib/categories'

/**
 * Returns a function that makes `source` a member of exactly the categories
 * `chosenIds`, one members PUT per changed category, then refetches the
 * categories. Members are read fresh first, so other members stay as they
 * are now.
 */
export function useSaveSourceCategories(): (
  source: SourceRef,
  chosenIds: string[],
) => Promise<void> {
  const queryClient = useQueryClient()
  return useCallback(
    async (source: SourceRef, chosenIds: string[]) => {
      try {
        const { categories } = await queryClient.fetchQuery({
          queryKey: queryKeys.categories,
          queryFn: listCategories,
        })
        await Promise.all(
          membershipChanges(categories, source, chosenIds).map(({ id, members }) =>
            setCategoryMembers(id, members),
          ),
        )
      } finally {
        await queryClient.invalidateQueries({ queryKey: queryKeys.categories })
      }
    },
    [queryClient],
  )
}
