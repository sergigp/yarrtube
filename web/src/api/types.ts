// Response shapes of the Rust HTTP API, mirroring the serde DTOs under
// `src/application/http/*/dto.rs`. Timestamps are ISO 8601 strings.

export type VideoStatus =
  | 'PENDING'
  | 'IN_PROGRESS'
  | 'DOWNLOADED'
  | 'ERRORED_RETRYING'
  | 'ERRORED'

export type VideoQuality = 'high' | 'mid' | 'low'

/** An entry of `GET /api/channels`. */
export interface ChannelListItem {
  id: string
  name: string
  path: string
  quality: VideoQuality
  video_limit: number
  reconcile_interval_minutes: number
  avatar_filename: string | null
  /** Whether the channel's videos get an AI summary. */
  summaries_enabled: boolean
  /** Whether AI sorts the channel's videos into categories. */
  categorization_enabled: boolean
  unwatched_count: number
  /** Still downloading or fetching something: it can't be moved yet. */
  work_in_progress: boolean
}

/** An entry of `GET /api/playlists`. */
export interface PlaylistListItem {
  id: string
  name: string
  path: string
  quality: string
  kind: string
  exclude_from_home: boolean
  hide_unwatched_count: boolean
  reconcile_interval_minutes: number
  /** Whether the playlist's videos get an AI summary. */
  summaries_enabled: boolean
  /** Whether AI sorts the playlist's videos into categories. */
  categorization_enabled: boolean
  created_at: string
  unwatched_count: number
  /** Still downloading or fetching something: it can't be moved yet. */
  work_in_progress: boolean
}

/** A sidebar/detail library entry: what channels and playlists share. */
export interface LibraryItem {
  id: string
  name: string
  path: string
  avatar_filename?: string | null
  unwatched_count?: number
  exclude_from_home?: boolean
  hide_unwatched_count?: boolean
}

/** An entry of `GET /api/playlists/:id/videos` and `/api/channels/:handle/videos`. */
export interface Video {
  id: string
  title: string
  status: VideoStatus
  quality: string | null
  filename: string | null
  thumbnail_filename: string | null
  duration_seconds: number | null
  created_at: string
  updated_at: string
  watched: boolean
  position_seconds: number
  synced_at: string | null
  published_at: string | null
  channel_name: string | null
  /** The fetched subtitles' language code (e.g. `en`), null until fetched. */
  subtitle_language: string | null
  /** YouTube chapters in playback order; empty when none or not checked yet. */
  chapters: Chapter[]
  /** The AI summary; null when the video has none or it is hidden. */
  summary: string | null
}

/** One of a video's chapters, in whole seconds from the start of the video. */
export interface Chapter {
  start_seconds: number
  end_seconds: number
  title: string
}

export interface HomeVideoSource {
  kind: 'playlist' | 'channel'
  id: string
  name: string
  path: string
  avatar_filename: string | null
}

export interface HomeVideo {
  id: string
  title: string
  thumbnail_filename: string | null
  duration_seconds: number | null
  watched: boolean
  position_seconds: number
  /** When YouTube published the video (RFC 3339), `null` when unknown. */
  published_at: string | null
  source: HomeVideoSource
  /** Categories having the YouTube video, in category-name order. */
  category_ids: string[]
}

/** Response of `GET /api/videos/search`. */
export interface VideoSearchResults {
  videos: HomeVideo[]
}

/** An entry of `GET /api/categories/:id/videos`: a source's video, with where it's listed from. */
export interface CategoryVideo extends Video {
  source: HomeVideoSource
  /** Categories having the YouTube video, in category-name order. */
  category_ids: string[]
}

/** A "More about <name>" home section: a category and its pending videos. */
export interface HomeCategorySection {
  id: string
  name: string
  color: string
  videos: HomeVideo[]
}

/** Response of `GET /api/videos/home`. */
export interface HomeVideos {
  continue_watching: HomeVideo[]
  categories: HomeCategorySection[]
  quick_watches: HomeVideo[]
  latest: HomeVideo[]
}

/** An entry of `GET /api/tasks`. `status` is `pending` or `running`. */
export interface Task {
  id: number
  task_type: string
  status: string
  retries: number
  run_at: string
  created_at: string
  last_error: string | null
  payload: Record<string, string>
}

/** Response of `GET /api/directories`. `root` is the absolute videos root. */
export interface DirectoryListing {
  root: string
  path: string
  entries: { name: string }[]
}

/** Response of `POST /api/channels`. */
export interface CreatedChannel {
  id: string
  name: string
  youtube_channel_id: string
  quality: string
  video_limit: number
  reconcile_interval_minutes: number
  path: string
  avatar_filename: string | null
  summaries_enabled: boolean
  categorization_enabled: boolean
  created_at: string
}

/** Response of `GET /api/playlists/preview`. */
export interface PlaylistPreview {
  id: string
  title: string
  video_count: number
}

/** Response of `GET /api/channels/preview`. */
export interface ChannelPreview {
  id: string
  title: string
  avatar_url: string | null
}

export interface VideoProgress {
  position_seconds: number
  duration_seconds?: number
  /** Whether the video was watched when the current playback session began. */
  was_watched: boolean
}

/** Response of `POST /api/videos/:id/progress`. */
export interface RecordProgressResponse {
  watched: boolean
}

/** An entry of the repository's `announcements/announcements.json`. */
export interface Announcement {
  id: string
  text: string
}

/** Which kind of build the daemon runs: a stable release, an unreleased `main` build, or a local one. */
export type BuildChannel = 'release' | 'edge' | 'dev'

/** The commit a build was made from. `date` is an RFC 3339 commit date. */
export interface BuildCommit {
  sha: string
  subject: string
  date: string
}

/** Response of `GET /api/version`. */
export interface AppVersion {
  version: string
  channel: BuildChannel
  /** Null for local builds. */
  commit: BuildCommit | null
}

/** An entry of `GET /api/categories`. Members are channel handles and playlist IDs. */
export interface Category {
  id: string
  name: string
  /** Lowercase `#rrggbb`. */
  color: string
  /** What the user wrote to describe the videos it should hold. */
  ai_prompt: string | null
  unwatched_count: number
  channels: string[]
  playlists: string[]
}

/** Response of `GET /api/categories`. */
export interface CategoryList {
  categories: Category[]
  /** Distinct unwatched videos across every category. */
  unwatched_total: number
}

/** Body of `PUT /api/categories/:id/members`: the category's members, exactly. */
export interface CategoryMembers {
  channels: string[]
  playlists: string[]
}

/** Body of `POST /api/categories`. */
export interface NewCategory extends CategoryMembers {
  name: string
  color: string
  ai_prompt?: string
}

/** Response of `POST /api/categories`. */
export interface CreatedCategory {
  id: string
  name: string
  color: string
  ai_prompt: string | null
  channels: string[]
  playlists: string[]
}

/** Response of `GET`/`PATCH /api/preferences`: every preference. */
export interface Preferences {
  categories_enabled: boolean
  home_show_continue_watching: boolean
  home_show_categories: boolean
  home_show_quick_watches: boolean
  home_show_latest: boolean
  theme: 'light' | 'dark'
  ai_enabled: boolean
  ollama_url: string | null
  summaries_enabled: boolean
  summaries_model: string
  summaries_context_length: number
  summaries_temperature: number
  summaries_thinking: boolean
  /** The saved prompt, or the shipped one when none is saved. */
  summaries_prompt: string
  summaries_prompt_is_default: boolean
  categorization_enabled: boolean
  categorization_model: string
  categorization_context_length: number
  categorization_temperature: number
  categorization_thinking: boolean
  /** The saved prompt, or the shipped one when none is saved. */
  categorization_prompt: string
  categorization_prompt_is_default: boolean
}

/**
 * Body of `PATCH /api/preferences`: only the preferences to change.
 * `ollama_url: ""` unsets the URL; `summaries_prompt: null` and
 * `categorization_prompt: null` go back to the shipped prompt.
 */
export type PreferencesPatch = Partial<
  Omit<
    Preferences,
    | 'ollama_url'
    | 'summaries_prompt'
    | 'summaries_prompt_is_default'
    | 'categorization_prompt'
    | 'categorization_prompt_is_default'
  >
> & {
  ollama_url?: string
  summaries_prompt?: string | null
  categorization_prompt?: string | null
}

/** Response of `GET /api/ollama?url=`: an Ollama server and its models. */
export interface OllamaServer {
  version: string
  models: OllamaModel[]
}

export interface OllamaModel {
  name: string
  size_bytes: number
  /** Whether the model can think before answering. */
  thinking: boolean
}
