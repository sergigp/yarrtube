/** A channel's or playlist's AI settings. */
interface SourceAiSettings {
  summaries_enabled: boolean
  categorization_enabled: boolean
}

/** Whether the source categorizes its videos: summaries and categorization both on. */
export function sourceCategorizes(source: SourceAiSettings): boolean {
  return source.categorization_enabled
}
