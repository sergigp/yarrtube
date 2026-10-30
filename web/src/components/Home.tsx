import { useMemo } from 'react'
import { Link } from 'react-router-dom'
import { useCategories, useCategoriesShown, usePreferences, useRecentVideos } from '@/api/queries'
import type { HomeVideo } from '@/api/types'
import type { VideoGridLayout } from '@/lib/videoGridLayout'
import type { CategoryChip } from './CategoryChips'
import { VideoGrid } from './VideoGrid'
import { Busy, VideoGridSkeleton } from './VideoSkeletons'

interface HomeSectionProps {
  title: string
  /** Where the heading and a "See all" link lead, when the section links anywhere. */
  href?: string
  name: string
  videos: HomeVideo[] | undefined
  error: Error | null
  layout: VideoGridLayout
  /** Category chips for the cards, on the sections mixing categories. */
  chips?: Map<string, CategoryChip>
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
  layout,
  chips,
  hideWhenEmpty,
}: HomeSectionProps) {
  if (hideWhenEmpty && (error || !videos || videos.length === 0)) {
    return null
  }

  return (
    <section className="min-w-0">
      <div className="mb-4 flex items-baseline justify-between gap-4">
        <h2 className="font-heading text-xl font-bold tracking-tight text-foreground">
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
      {!error && videos && <VideoGrid videos={videos} chips={chips} layout={layout} />}
    </section>
  )
}

/** Stands in for the sections while the home videos load: two shelves over a grid. */
function HomeSkeleton() {
  return (
    <Busy>
      <div className="flex flex-col gap-5 sm:gap-10">
        {[0, 1].map((shelf) => (
          <div key={shelf} className="flex flex-col gap-4">
            <div aria-hidden className="h-7 w-48 animate-pulse rounded-md bg-muted" />
            <VideoGridSkeleton count={6} layout="shelf" announced={false} />
          </div>
        ))}
        <div className="flex flex-col gap-4">
          <div aria-hidden className="h-7 w-40 animate-pulse rounded-md bg-muted" />
          <VideoGridSkeleton count={12} layout="feed" announced={false} />
        </div>
      </div>
    </Busy>
  )
}

/**
 * What home says in place of a turned-off "Latest videos": the load error, or
 * that nothing shows once every other section turned out empty.
 */
function NothingElseShown({
  error,
  othersHaveVideos,
  loaded,
}: {
  error: Error | null
  othersHaveVideos: boolean
  loaded: boolean
}) {
  if (error) {
    return <p className="text-sm text-destructive">Failed to load videos: {error.message}</p>
  }
  if (!loaded || othersHaveVideos) {
    return null
  }
  return (
    <p className="text-sm text-muted-foreground">
      Nothing to show here.{' '}
      <Link to="/preferences" className="underline hover:text-foreground">
        Preferences
      </Link>
    </p>
  )
}

export function Home() {
  const { data: home, error } = useRecentVideos()
  const { data: categories } = useCategories()
  const { data: preferences } = usePreferences()
  // Category sections and chips show only while categories do, even when the
  // cached response is from before they were turned off.
  const withCategories = useCategoriesShown()
  const categorySections = (withCategories && home?.categories) || []
  const chips = useMemo(
    () =>
      withCategories
        ? new Map(
            (categories?.categories ?? []).map((category) => [
              category.id,
              { name: category.name, color: category.color },
            ]),
          )
        : undefined,
    [categories, withCategories],
  )
  // Shown until the preferences say otherwise, as before they existed.
  const showLatest = preferences?.home_show_latest !== false
  // "Latest videos" stands in for an empty home, so it only hides when
  // another section has something to show. A failed refresh keeps the last
  // response cached, but those sections hide on error, so Latest shows it.
  const othersHaveVideos =
    !!home &&
    !error &&
    (home.continue_watching.length > 0 ||
      categorySections.some((category) => category.videos.length > 0) ||
      home.quick_watches.length > 0)

  if (!home && !error) {
    return <HomeSkeleton />
  }

  return (
    <div className="flex min-w-0 flex-col gap-5 sm:gap-10">
      <HomeSection
        title="Continue watching"
        name="continue watching videos"
        videos={home?.continue_watching}
        error={error}
        layout="shelf"
        chips={chips}
        hideWhenEmpty
      />
      {categorySections.map((category) => (
        <HomeSection
          key={category.id}
          title={`More about ${category.name}`}
          href={`/categories/${encodeURIComponent(category.id)}`}
          name={`${category.name} videos`}
          videos={category.videos}
          error={error}
          layout="shelf"
          hideWhenEmpty
        />
      ))}
      <HomeSection
        title="Quick watches"
        name="quick watches"
        videos={home?.quick_watches}
        error={error}
        layout="shelf"
        chips={chips}
        hideWhenEmpty
      />
      {showLatest ? (
        <HomeSection
          title="Latest videos"
          name="recent videos"
          videos={home?.latest}
          error={error}
          layout="feed"
          chips={chips}
          hideWhenEmpty={othersHaveVideos}
        />
      ) : (
        <NothingElseShown error={error} othersHaveVideos={othersHaveVideos} loaded={!!home} />
      )}
    </div>
  )
}
