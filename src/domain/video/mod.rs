pub mod channel_episode_number;
pub mod episode_number;
pub mod episode_paths;
pub mod errors;
pub mod events;
pub mod home_limits;
pub mod home_video_view;
pub mod home_videos;
pub mod playback_position;
pub mod playlist_episode_number;
pub mod sourced_video;
pub mod subtitle_state;
#[allow(clippy::module_inception)]
pub mod video;
pub mod video_duration;
pub mod video_filename;
pub mod video_id;
pub mod video_output_entry;
pub mod video_record_id;
pub mod video_status;
pub mod video_view;

pub use channel_episode_number::ChannelEpisodeNumber;
pub use episode_number::EpisodeNumber;
pub use episode_paths::EpisodePaths;
pub use errors::{ListVideosError, UpdateWatchStateError};
pub use events::{VideoDownloaded, VideoSubtitlesFetched};
pub use home_limits::HomeLimits;
pub use home_video_view::HomeVideoView;
pub use home_videos::HomeVideos;
pub use playback_position::PlaybackPosition;
pub use playlist_episode_number::PlaylistEpisodeNumber;
pub use sourced_video::{SourcedVideo, VideoSource};
pub use subtitle_state::SubtitleState;
pub use video::Video;
pub use video_duration::VideoDuration;
pub use video_id::VideoId;
pub use video_output_entry::{
    MOVIE_NFO_FILENAME, SHOW_NFO_FILENAME, SHOW_POSTER_FILENAME, STAGING_DIRNAME, is_episode_path,
    is_season_folder, nfo_path_for, owning_entry, resolve_output_dir, staging_dir,
    subtitle_staging_dir, top_level_entry,
};
pub use video_record_id::VideoRecordId;
pub use video_status::VideoStatus;
pub use video_view::VideoView;
