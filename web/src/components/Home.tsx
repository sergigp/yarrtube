import { useMemo } from 'react'
import { useCategories, useRecentVideos } from '@/api/queries'
import type { HomeVideo } from '@/api/types'
import { VideoGrid } from './VideoGrid'

interface HomeSectionProps {
  title: string
  name: string
  videos: HomeVideo[] | undefined
  error: Error | null
  categoryColors: Map<string, string>
  hideWhenEmpty?: boolean
}

/**
 * One home section: a heading over a grid of video cards. With `hideWhenEmpty`
 * the whole section, heading included, renders nothing unless it has videos
 * to show; otherwise it reports loading, errors and emptiness in place.
 */
function HomeSection({
  title,
  name,
  videos,
  error,
  categoryColors,
  hideWhenEmpty,
}: HomeSectionProps) {
  if (hideWhenEmpty && (error || !videos || videos.length === 0)) {
    return null
  }

  return (
    <section className="min-w-0">
      <h2 className="mb-4 font-heading text-lg font-semibold text-foreground">{title}</h2>
      {error && (
        <p className="text-sm text-destructive">
          Failed to load {name}: {error.message}
        </p>
      )}
      {!error && !videos && <p className="text-sm text-muted-foreground">Loading {name}…</p>}
      {!error && videos && <VideoGrid videos={videos} categoryColors={categoryColors} />}
    </section>
  )
}

export function Home() {
  const { data: home, error } = useRecentVideos()
  const { data: categories } = useCategories()
  const categoryColors = useMemo(
    () => new Map((categories?.categories ?? []).map((category) => [category.id, category.color])),
    [categories],
  )

  return (
    <div className="flex min-w-0 flex-col gap-10">
      <HomeSection
        title="Continue watching"
        name="continue watching videos"
        videos={home?.continue_watching}
        error={error}
        categoryColors={categoryColors}
        hideWhenEmpty
      />
      {home?.categories.map((category) => (
        <HomeSection
          key={category.id}
          title={`More about ${category.name}`}
          name={`${category.name} videos`}
          videos={category.videos}
          error={null}
          categoryColors={categoryColors}
          hideWhenEmpty
        />
      ))}
      <HomeSection
        title="Quick watches"
        name="quick watches"
        videos={home?.quick_watches}
        error={error}
        categoryColors={categoryColors}
        hideWhenEmpty
      />
      <HomeSection
        title="Latest videos"
        name="recent videos"
        videos={home?.latest}
        error={error}
        categoryColors={categoryColors}
      />
    </div>
  )
}
