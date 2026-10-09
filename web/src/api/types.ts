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
  avatar_filename: string | null
  unwatched_count: number
}

/** An entry of `GET /api/playlists`. Playlists don't report an unwatched count. */
export interface PlaylistListItem {
  id: string
  name: string
  path: string
  quality: string
  kind: string
  exclude_from_home: boolean
  created_at: string
}

/** A sidebar/detail library entry: what channels and playlists share. */
export interface LibraryItem {
  id: string
  name: string
  path: string
  avatar_filename?: string | null
  unwatched_count?: number
  exclude_from_home?: boolean
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
  description: string | null
  channel_name: string | null
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
  source: HomeVideoSource
}

/** Response of `GET /api/videos/home`. */
export interface HomeVideos {
  continue_watching: HomeVideo[]
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
  path: string
  avatar_filename: string | null
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
