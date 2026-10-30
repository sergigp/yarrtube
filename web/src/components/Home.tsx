import { useMemo } from 'react'
import { Link } from 'react-router-dom'
import { useCategories, usePreferences, useRecentVideos } from '@/api/queries'
import { homeTintShown } from '@/lib/preferences'
import type { HomeVideo } from '@/api/types'
import { VideoGrid } from './VideoGrid'

interface HomeSectionProps {
  title: string
  /** Where the heading and a "See all" link lead, when the section links anywhere. */
  href?: string
  name: string
  videos: HomeVideo[] | undefined
  error: Error | null
  /** Category id → color, when cards are tinted. */
  categoryColors: Map<string, string> | undefined
  hideWhenEmpty?: boolean
}

/**
 * One home section: a heading over a grid of video cards. With `hideWhenEmpty`
 * the whole section, heading included, renders nothing unless it has videos
 * to show; otherwise it reports loading, errors and emptiness in place.
 */
function HomeSection({
  title,
  href,
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
      <div className="mb-4 flex items-baseline justify-between gap-4">
        <h2 className="font-heading text-lg font-semibold text-foreground">
          {href ? (
            <Link to={href} className="hover:underline">
              {title}
            </Link>
          ) : (
            title
          )}
        </h2>
        {href && (
          <Link to={href} className="shrink-0 text-sm text-muted-foreground hover:text-foreground">
            See all
          </Link>
        )}
      </div>
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
  const { data: preferences, isError: preferencesFailed } = usePreferences()
  const tinted = homeTintShown(preferences, preferencesFailed)
  const categoryColors = useMemo(
    () =>
      tinted
        ? new Map((categories?.categories ?? []).map((category) => [category.id, category.color]))
        : undefined,
    [categories, tinted],
  )
  // "Latest videos" stands in for an empty home, so it only hides when
  // another section has something to show. A failed refresh keeps the last
  // response cached, but those sections hide on error, so Latest shows it.
  const othersHaveVideos =
    !!home &&
    !error &&
    (home.continue_watching.length > 0 ||
      home.categories.some((category) => category.videos.length > 0) ||
      home.quick_watches.length > 0)

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
          href={`/categories/${encodeURIComponent(category.id)}`}
          name={`${category.name} videos`}
          videos={category.videos}
          error={error}
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
        hideWhenEmpty={othersHaveVideos}
      />
    </div>
  )
}
