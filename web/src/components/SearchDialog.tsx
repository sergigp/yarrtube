import { useId, useMemo, useState, type KeyboardEvent, type ReactNode } from 'react'
import { useNavigate } from 'react-router-dom'
import { ListVideo, Search } from 'lucide-react'
import { avatarMediaUrl, videoMediaUrl } from '@/api/client'
import {
  VIDEO_SEARCH_MIN_LENGTH,
  useCategories,
  useCategoriesShown,
  useChannels,
  usePlaylists,
  useVideoSearch,
} from '@/api/queries'
import type { HomeVideo } from '@/api/types'
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog'
import { useDebouncedValue } from '@/hooks/useDebouncedValue'
import { searchByName } from '@/lib/search'
import { videoPathInSource } from '@/lib/videoPath'
import { cn } from '@/lib/utils'
import { Thumbnail } from './Thumbnail'

/** How long typing must pause before videos are searched. */
const VIDEO_SEARCH_DEBOUNCE_MS = 250

/** One choosable result: where it leads and how it shows. */
interface Result {
  key: string
  path: string
  content: ReactNode
}

interface SearchDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

/**
 * Global search: categories, channels and playlists by name as the user
 * types, and downloaded videos by title once the text has 2 characters and
 * typing pauses. Arrow keys move through the results, Enter or a click opens
 * one and closes the dialog. Full screen on phones.
 */
export function SearchDialog({ open, onOpenChange }: SearchDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        showCloseButton={false}
        className="top-[12vh] translate-y-0 gap-0 overflow-hidden p-0 sm:max-w-xl max-sm:inset-0 max-sm:top-0 max-sm:left-0 max-sm:flex max-sm:max-w-none max-sm:translate-x-0 max-sm:flex-col max-sm:rounded-none max-sm:border-0"
      >
        <DialogTitle className="sr-only">Search</DialogTitle>
        <DialogDescription className="sr-only">
          Find categories, channels, playlists and videos.
        </DialogDescription>
        {/* Remounted on every open, so it starts empty. */}
        {open && <SearchPanel onChosen={() => onOpenChange(false)} />}
      </DialogContent>
    </Dialog>
  )
}

function SearchPanel({ onChosen }: { onChosen: () => void }) {
  const navigate = useNavigate()
  const [text, setText] = useState('')
  const [activeIndex, setActiveIndex] = useState(0)
  const debouncedText = useDebouncedValue(text, VIDEO_SEARCH_DEBOUNCE_MS)
  const searchesVideos = text.trim().length >= VIDEO_SEARCH_MIN_LENGTH
  const videoSearch = useVideoSearch(searchesVideos ? debouncedText : '')
  const { data: categoryList } = useCategories()
  // No category is found while categories are turned off.
  const categoriesShown = useCategoriesShown()
  const { data: channels } = useChannels()
  const { data: playlists } = usePlaylists()
  const listboxId = useId()
  const optionId = (index: number) => `${listboxId}-option-${index}`

  const nameGroups = useMemo(
    () => [
      {
        name: 'Categories',
        results: searchByName(categoriesShown ? (categoryList?.categories ?? []) : [], text).map(
          (category) => ({
            key: `category:${category.id}`,
            path: `/categories/${encodeURIComponent(category.id)}`,
            content: (
              <>
                <span
                  aria-hidden="true"
                  className="size-2.5 shrink-0 rounded-full"
                  style={{ background: category.color }}
                />
                <span className="truncate">{category.name}</span>
              </>
            ),
          }),
        ),
      },
      {
        name: 'Channels',
        results: searchByName(channels ?? [], text).map((channel) => ({
          key: `channel:${channel.id}`,
          path: `/channels/${channel.id}`,
          content: (
            <>
              <Thumbnail
                src={channel.avatar_filename ? avatarMediaUrl(channel.avatar_filename) : null}
                className="size-5 shrink-0 rounded-full object-cover"
              />
              <span className="truncate">{channel.name}</span>
            </>
          ),
        })),
      },
      {
        name: 'Playlists',
        results: searchByName(playlists ?? [], text).map((playlist) => ({
          key: `playlist:${playlist.id}`,
          path: `/playlists/${playlist.id}`,
          content: (
            <>
              <ListVideo aria-hidden="true" className="size-4 shrink-0 text-muted-foreground" />
              <span className="truncate">{playlist.name}</span>
            </>
          ),
        })),
      },
    ],
    [categoryList, categoriesShown, channels, playlists, text],
  )
  // Results for older text never show for newer text.
  const videos =
    searchesVideos && debouncedText.trim() === text.trim() ? videoSearch.data?.videos : undefined
  const videoResults: Result[] = (videos ?? []).map(videoResult)
  const videoStatus = !searchesVideos
    ? null
    : videoSearch.isError
      ? 'Videos could not be searched.'
      : videos === undefined
        ? 'Searching…'
        : null
  const groups = [
    ...nameGroups,
    { name: 'Videos', results: videoResults, status: videoStatus },
  ].filter((group) => group.results.length > 0 || ('status' in group && group.status))
  const results = groups.flatMap((group) => group.results)
  const active = Math.min(activeIndex, results.length - 1)

  const choose = (result: Result | undefined) => {
    if (!result) {
      return
    }
    onChosen()
    navigate(result.path)
  }

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault()
      const step = event.key === 'ArrowDown' ? 1 : -1
      setActiveIndex(Math.max(0, Math.min(results.length - 1, active + step)))
    } else if (event.key === 'Enter') {
      event.preventDefault()
      choose(results[active])
    }
  }

  let index = -1
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex items-center gap-2 border-b border-border px-4">
        <Search aria-hidden="true" className="size-4 shrink-0 text-muted-foreground" />
        <input
          autoFocus
          role="combobox"
          aria-label="Search"
          aria-expanded={results.length > 0}
          aria-controls={listboxId}
          aria-autocomplete="list"
          aria-activedescendant={results.length > 0 ? optionId(active) : undefined}
          value={text}
          onChange={(event) => {
            setText(event.target.value)
            setActiveIndex(0)
          }}
          onKeyDown={onKeyDown}
          placeholder="Search categories, channels, playlists and videos"
          className="h-12 min-w-0 flex-1 bg-transparent text-base outline-none placeholder:text-muted-foreground sm:text-sm"
        />
      </div>
      <div
        id={listboxId}
        role="listbox"
        aria-label="Results"
        className="max-h-[60vh] overflow-y-auto p-2 empty:hidden max-sm:max-h-none max-sm:flex-1"
      >
        {groups.map((group) => (
          <div key={group.name} role="group" aria-label={group.name} className="py-1">
            <div aria-hidden="true" className="px-2 py-1 text-xs font-medium text-muted-foreground">
              {group.name}
            </div>
            {group.results.map((result) => {
              index += 1
              const resultIndex = index
              return (
                <div
                  key={result.key}
                  id={optionId(resultIndex)}
                  role="option"
                  aria-selected={resultIndex === active}
                  onMouseMove={() => setActiveIndex(resultIndex)}
                  onClick={() => choose(result)}
                  className={cn(
                    'flex cursor-pointer items-center gap-2 rounded-md px-2 py-1.5 text-sm',
                    resultIndex === active && 'bg-accent',
                  )}
                >
                  {result.content}
                </div>
              )
            })}
            {'status' in group && group.status && (
              <p className="px-2 py-1.5 text-sm text-muted-foreground">{group.status}</p>
            )}
          </div>
        ))}
      </div>
      {text.trim() && groups.length === 0 && (
        <p className="px-4 py-6 text-center text-sm text-muted-foreground">
          Nothing matches "{text.trim()}".
        </p>
      )}
    </div>
  )
}

function videoResult(video: HomeVideo): Result {
  return {
    key: `video:${video.source.kind}:${video.source.id}:${video.id}`,
    path: videoPathInSource(video),
    content: (
      <>
        <Thumbnail
          src={
            video.thumbnail_filename
              ? videoMediaUrl(video.source.path, video.thumbnail_filename)
              : null
          }
          className="aspect-video w-16 shrink-0 rounded object-cover"
        />
        <span className="flex min-w-0 flex-col">
          <span className="truncate">{video.title}</span>
          <span className="truncate text-xs text-muted-foreground">{video.source.name}</span>
        </span>
      </>
    ),
  }
}
