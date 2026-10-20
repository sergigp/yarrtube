import { useState } from 'react'
import { useCategories } from '@/api/queries'
import { categoryIdsOf, type SourceRef } from '@/lib/categories'
import { useSaveSourceCategories } from './useSaveSourceCategories'

export interface SourceCategoriesField {
  /** The chosen categories: the source's current ones until changed. */
  value: string[]
  onChange: (ids: string[]) => void
  /** Whether the chosen categories differ from the source's current ones. */
  changed: boolean
  /** Saves the chosen categories when they changed. */
  save: () => Promise<void>
}

/** An edit dialog's Categories field for `source`, starting from its current categories. */
export function useSourceCategoriesField(source: SourceRef): SourceCategoriesField {
  const { data } = useCategories()
  const saveSourceCategories = useSaveSourceCategories()
  const [chosen, setChosen] = useState<string[] | null>(null)
  const current = data ? categoryIdsOf(data.categories, source) : []
  const value = chosen ?? current
  const changed =
    chosen !== null &&
    (chosen.length !== current.length || chosen.some((id) => !current.includes(id)))

  return {
    value,
    onChange: setChosen,
    changed,
    save: () => (changed ? saveSourceCategories(source, value) : Promise.resolve()),
  }
}
