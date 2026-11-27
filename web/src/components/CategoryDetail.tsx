import { useEffect, useId, useState } from 'react'
import { Link, useLocation, useNavigate, useParams, useSearchParams } from 'react-router-dom'
import { FolderPlus, Plus, SlidersHorizontal } from 'lucide-react'
import { avatarMediaUrl, deleteCategory } from '@/api/client'
import {
  useCategories,
  useCategoryVideos,
  useLibraryAction,
  useMarkCategoryWatched,
  usePreferences,
} from '@/api/queries'
import type { Category, CategoryVideo } from '@/api/types'
import { useRememberedChoice } from '@/hooks/useRememberedChoice'
import { useVideoSelection } from '@/hooks/useVideoSelection'
import { deleteCategoryDescription, memberCount } from '@/lib/categories'
import {
  VIDEO_SORT_LABELS,
  WATCH_FILTERS,
  matchesWatchFilter,
  newestUnwatched,
  sortVideos,
  sourceKey,
  sourceSummary,
  sourcesSummary,
  videoSources,
  watchFilterCounts,
  type VideoSort,
  type VideoSourceOption,
} from '@/lib/sourceVideos'
import { categoryPath } from '@/lib/sourcePath'
import { videoPathIn } from '@/lib/videoPath'
import { Button } from '@/components/ui/button'
import { CategoryPlayer } from './CategoryPlayer'
import { DetailHeader } from './DetailHeader'
import { CategoryDialog } from './CategoryDialog'
import { PlayLink } from './PlayLink'
import { PlaylistIcon } from './PlaylistIcon'
import { Thumbnail } from './Thumbnail'
import { VideoGrid } from './VideoGrid'
import { ChoiceMenu, VideoListToolbar } from './VideoListToolbar'
import { VideoGridSkeleton } from './VideoSkeletons'

const CATEGORY_SORTS: readonly VideoSort[] = ['newest', 'oldest']



/** Router state that opens the category settings dialog on arrival at a category's view. */
export interface CategorySettingsState {
  editSettings: true
}

/**
 * What an empty category shows instead of videos: what it is for, and the
 * way to fill it, the category settings dialog.
 */
function EmptyCategory({ category, onAddSources }: { category: Category; onAddSources: () => void }) {
  const headingId = useId()
  return (
    <section
      aria-labelledby={headingId}
      className="flex flex-col items-center gap-4 rounded-xl border border-dashed border-border px-6 py-16 text-center"
    >
      <span className="flex size-12 items-center justify-center rounded-full bg-secondary text-muted-foreground">
        <FolderPlus className="size-6" />
      </span>
      <div className="flex max-w-sm flex-col gap-1.5">
        <h2 id={headingId} className="font-heading text-lg font-semibold">
          Nothing in {category.name} yet
        </h2>
        <p className="text-sm text-muted-foreground">
          Add channels and playlists to {category.name} to watch their videos here, newest first.
        </p>
      </div>
      <Button type="button" onClick={onAddSources}>
        <Plus />
        Add channels and playlists
      </Button>
    </section>
  )
}

/**
 * A category's view (`/categories/:id`): its header ("Play newest
 * unwatched", Edit category, ⋮) and videos as a grid, under one toolbar of
 * watch-state chips, a source picker and the sort, all remembered per
 * category; or, once one is picked (`?video=`), playing it with the
 * category's videos, in the remembered sort, as the queue beside the player.
 */
export function CategoryDetail() {
  const { id = '' } = useParams()
  const [searchParams] = useSearchParams()
  const playing = searchParams.has('video')
  const navigate = useNavigate()
  const location = useLocation()
  const refreshing = useLibraryAction()
  const markCategoryWatched = useMarkCategoryWatched()
  const [editingSettings, setEditingSettings] = useState(false)
  // Arriving with CategorySettingsState opens the dialog once per navigation;
  // the state is then dropped from history so reload and back don't reopen it.
  const arrivedEditing = (location.state as CategorySettingsState | null)?.editSettings === true
  const [openedFor, setOpenedFor] = useState<string | null>(null)
  if (arrivedEditing && openedFor !== location.key) {
    setOpenedFor(location.key)
    setEditingSettings(true)
  }
  useEffect(() => {
    if (arrivedEditing) {
      navigate(location.pathname, { replace: true, state: null })
    }
  }, [arrivedEditing, location.pathname, navigate])
  const { data } = useCategories()
  const { data: videos, error, isFetchedAfterMount: videosFresh } = useCategoryVideos(id)
  const { data: preferences } = usePreferences()
  const category = data?.categories.find((candidate) => candidate.id === id)
  const [sort, setSort] = useRememberedChoice(
    `yarrtube.category.${id}.sort`,
    CATEGORY_SORTS,
    'newest',
  )
  const queue = videos ? sortVideos(videos, sort) : undefined
  const selection = useVideoSelection(queue)

  if (preferences && !preferences.categories_enabled) {
    return (
      <p className="text-sm text-muted-foreground">
        Categories are turned off.{' '}
        <Link to="/settings/general" className="underline hover:text-foreground">
          Settings
        </Link>
      </p>
    )
  }

  if (!data) {
    return <VideoGridSkeleton count={12} />
  }
  if (!category) {
    return <p className="text-sm text-muted-foreground">Category not found.</p>
  }
  const sources = memberCount(category)
  // Videos set by hand or by the AI fill a category without sources too.
  const empty = sources === 0 && videos?.length === 0
  const hasVideos = sources > 0 || (videos?.length ?? 0) > 0
  const next = videos ? newestUnwatched(videos) : null
  const sourceOptions = videos ? videoSources(videos) : []
  // What the page lists, playlists whose count is hidden included, like its
  // "Unwatched" chip; the category's own count leaves those out, for the sidebar.
  const unwatchedCount = videos ? watchFilterCounts(videos).unwatched : category.unwatched_count

  return (
    <div className={playing ? 'flex flex-col md:h-full md:min-h-[480px]' : 'flex min-w-0 flex-col'}>
      <DetailHeader
        name={category.name}
        badge={{ kind: 'color', color: category.color }}
        compact={playing}
        nameHref={playing ? categoryPath(category.id) : undefined}
        nameLinkLabel="Back to the category page"
        summary={
          videos
            ? [
                sourceSummary({ videoCount: videos.length, unwatchedCount }),
                sourcesSummary(sourceOptions),
              ]
                .filter(Boolean)
                .join(' · ')
            : null
        }
        primaryAction={
          !playing &&
          next && <PlayLink to={videoPathIn(categoryPath(category.id), next.id)} label="Play newest unwatched" />
        }
        unwatchedCount={unwatchedCount}
        extraActions={[
          {
            label: 'Edit category',
            icon: <SlidersHorizontal />,
            onSelect: () => setEditingSettings(true),
            iconOnDesktop: true,
          },
        ]}
        onMarkWatched={hasVideos ? () => markCategoryWatched(category.id) : undefined}
        onDelete={refreshing(async () => {
          await deleteCategory(category.id)
          navigate('/')
        })}
        deleteDescription={deleteCategoryDescription(category)}
      />
      {empty ? (
        <EmptyCategory category={category} onAddSources={() => setEditingSettings(true)} />
      ) : playing ? (
        <CategoryPlayer
          key={category.id}
          videos={queue}
          videosFresh={videosFresh}
          error={error}
          selection={selection}
          orderLabel={VIDEO_SORT_LABELS[sort].toLowerCase()}
        />
      ) : !videos ? (
        // A failed background refresh keeps the loaded videos on screen.
        error ? (
          <p className="text-sm text-destructive">Failed to load videos: {error.message}</p>
        ) : (
          <VideoGridSkeleton count={12} />
        )
      ) : (
        <CategoryBrowser
          category={category}
          videos={videos}
          sources={sourceOptions}
          sort={sort}
          onSortChange={setSort}
        />
      )}
      <CategoryDialog category={category} open={editingSettings} onOpenChange={setEditingSettings} />
    </div>
  )
}

/** The browsing body: chips, source picker and sort over the category's videos as cards. */
function CategoryBrowser({
  category,
  videos,
  sources,
  sort,
  onSortChange,
}: {
  category: Category
  videos: CategoryVideo[]
  sources: VideoSourceOption[]
  sort: VideoSort
  onSortChange: (sort: VideoSort) => void
}) {
  const [filter, setFilter] = useRememberedChoice(
    `yarrtube.category.${category.id}.filter`,
    WATCH_FILTERS,
    'all',
  )
  const [from, setFrom] = useRememberedChoice(
    `yarrtube.category.${category.id}.source`,
    ['all', ...sources.map((source) => source.key)],
    'all',
  )
  const fromSource = videos.filter((video) => from === 'all' || sourceKey(video.source) === from)
  const shown = sortVideos(
    fromSource.filter((video) => matchesWatchFilter(video, filter)),
    sort,
  )

  if (videos.length === 0) {
    return <VideoGrid videos={[]} />
  }

  return (
    <>
      <VideoListToolbar
        counts={watchFilterCounts(fromSource)}
        filter={filter}
        onFilterChange={setFilter}
        sorts={CATEGORY_SORTS}
        sort={sort}
        onSortChange={onSortChange}
      >
        {sources.length > 1 && (
          <ChoiceMenu
            label="From"
            prefix="From:"
            value={from}
            choices={[
              { value: 'all', label: 'All sources', count: videos.length },
              ...sources.map((source) => ({
                value: source.key,
                label: source.name,
                count: source.count,
                icon:
                  source.kind === 'channel' ? (
                    <Thumbnail
                      src={source.avatarFilename ? avatarMediaUrl(source.avatarFilename) : null}
                      className="block size-4.5 shrink-0 rounded-full object-cover"
                    />
                  ) : (
                    <PlaylistIcon className="size-4.5 rounded-[5px]" />
                  ),
              })),
            ]}
            onChange={setFrom}
          />
        )}
      </VideoListToolbar>
      {shown.length === 0 ? (
        <p className="text-sm text-muted-foreground">No videos match this filter.</p>
      ) : (
        <VideoGrid
          videos={shown}
          layout="feed"
          videoPathOf={(video) => videoPathIn(categoryPath(category.id), video.id)}
        />
      )}
    </>
  )
}

