import { useRecentVideos } from '@/api/queries'
import type { HomeVideo } from '@/api/types'
import { VideoGrid } from './VideoGrid'

interface HomeSectionProps {
  title: string
  name: string
  videos: HomeVideo[] | undefined
  error: Error | null
  hideWhenEmpty?: boolean
  showProgress?: boolean
}

/**
 * One home section: a heading over a grid of video cards. With `hideWhenEmpty`
 * the whole section, heading included, renders nothing unless it has videos
 * to show; otherwise it reports loading, errors and emptiness in place.
 */
function HomeSection({ title, name, videos, error, hideWhenEmpty, showProgress }: HomeSectionProps) {
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
      {!error && videos && <VideoGrid videos={videos} showProgress={showProgress} />}
    </section>
  )
}

export function Home() {
  const { data: home, error } = useRecentVideos()

  return (
    <div className="flex min-w-0 flex-col gap-10">
      <HomeSection
        title="Continue watching"
        name="continue watching videos"
        videos={home?.continue_watching}
        error={error}
        hideWhenEmpty
        showProgress
      />
      <HomeSection
        title="Quick watches"
        name="quick watches"
        videos={home?.quick_watches}
        error={error}
        hideWhenEmpty
      />
      <HomeSection title="Latest videos" name="recent videos" videos={home?.latest} error={error} />
    </div>
  )
}
