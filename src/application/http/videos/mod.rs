pub mod category_dto;
pub mod dto;

use super::blocking::run_blocking;
use super::error::ApiError;
use super::validation::{MISSING_POSITION, MISSING_SEARCH_TEXT, MISSING_WAS_WATCHED, required};
use crate::domain::category::CategoryId;
use crate::domain::channel::ChannelHandle;
use crate::domain::playlist::PlaylistId;
use crate::domain::services::{
    SubtitleReader, SubtitleReaderApi, VideoCategoryChoiceSearcher, VideoCategoryChoiceSearcherApi,
    VideoCategoryEditor, VideoCategoryEditorApi, VideoRetrier, VideoRetrierApi, VideoSearcher,
    VideoSearcherApi, VideoWatchStateUpdater, VideoWatchStateUpdaterApi,
};
use crate::domain::video::{
    HomeLimits, ListVideosError, PlaybackPosition, ReadSubtitlesError, RetryVideoError,
    UpdateWatchStateError, VideoDuration, VideoId, VideoSearchText, WebVtt,
};
use crate::domain::video_category::EditVideoCategoriesError;
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::http::header::{CONTENT_TYPE, HeaderName};
use category_dto::{INVALID_BELONGS, SetVideoCategoryRequest, VideoCategoriesResponse};
use dto::{
    CategoryVideoResponse, HomeResponse, HomeVideoResponse, RecordProgressRequest,
    RecordProgressResponse, SearchVideosQuery, SearchVideosResponse, VideoResponse,
};

/// A WebVTT body with its content type header.
pub type VttResponse = ([(HeaderName, &'static str); 1], String);

const VTT_CONTENT_TYPE: &str = "text/vtt; charset=utf-8";

const SEARCH_LIMIT: usize = 20;

const HOME_LIMITS: HomeLimits = HomeLimits {
    continue_watching: 6,
    category_sections: 3,
    category_videos: 6,
    quick_watches: 6,
    latest: 18,
};

pub async fn list_videos_for_playlist(
    State(video_searcher): State<VideoSearcher>,
    Path(playlist_id): Path<String>,
) -> Result<Json<Vec<VideoResponse>>, ApiError> {
    let playlist_id = PlaylistId::new(playlist_id)?;

    let videos = run_blocking(move || video_searcher.list_for_playlist(&playlist_id))
        .await?
        .map_err(list_videos_error)?;
    Ok(Json(videos.into_iter().map(VideoResponse::from).collect()))
}

pub async fn list_videos_for_channel(
    State(video_searcher): State<VideoSearcher>,
    Path(handle): Path<String>,
) -> Result<Json<Vec<VideoResponse>>, ApiError> {
    let channel_id = ChannelHandle::new(handle)?;

    let videos = run_blocking(move || video_searcher.list_for_channel(&channel_id))
        .await?
        .map_err(list_videos_error)?;
    Ok(Json(videos.into_iter().map(VideoResponse::from).collect()))
}

pub async fn list_videos_for_category(
    State(video_searcher): State<VideoSearcher>,
    Path(id): Path<String>,
) -> Result<Json<Vec<CategoryVideoResponse>>, ApiError> {
    let id = CategoryId::new(id)?;

    let videos = run_blocking(move || video_searcher.list_for_category(&id))
        .await?
        .map_err(list_videos_error)?;
    Ok(Json(
        videos
            .into_iter()
            .map(CategoryVideoResponse::from)
            .collect(),
    ))
}

pub async fn list_home_videos(
    State(video_searcher): State<VideoSearcher>,
) -> Result<Json<HomeResponse>, ApiError> {
    let home = run_blocking(move || video_searcher.list_home(HOME_LIMITS))
        .await?
        .map_err(list_videos_error)?;
    Ok(Json(HomeResponse::from(home)))
}

pub async fn search_videos(
    State(video_searcher): State<VideoSearcher>,
    Query(query): Query<SearchVideosQuery>,
) -> Result<Json<SearchVideosResponse>, ApiError> {
    let text = VideoSearchText::new(required(query.q, MISSING_SEARCH_TEXT)?)?;

    let videos = run_blocking(move || video_searcher.search_by_title(&text, SEARCH_LIMIT))
        .await?
        .map_err(list_videos_error)?;
    Ok(Json(SearchVideosResponse {
        videos: videos.into_iter().map(HomeVideoResponse::from).collect(),
    }))
}

pub async fn record_video_progress(
    State(video_watch_state_updater): State<VideoWatchStateUpdater>,
    Path(id): Path<String>,
    Json(request): Json<RecordProgressRequest>,
) -> Result<Json<RecordProgressResponse>, ApiError> {
    let id = VideoId::new(id)?;
    let position = PlaybackPosition::new(required(request.position_seconds, MISSING_POSITION)?)?;
    let reported_duration = request
        .duration_seconds
        .map(VideoDuration::new)
        .transpose()?;
    let was_watched = required(request.was_watched, MISSING_WAS_WATCHED)?;

    let watched = run_blocking(move || {
        video_watch_state_updater.update(&id, position, reported_duration, was_watched)
    })
    .await?
    .map_err(update_watch_state_error)?;

    Ok(Json(RecordProgressResponse { watched }))
}

pub async fn mark_video_watched(
    State(video_watch_state_updater): State<VideoWatchStateUpdater>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = VideoId::new(id)?;

    run_blocking(move || video_watch_state_updater.mark_video_watched(&id))
        .await?
        .map_err(update_watch_state_error)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_video_subtitles(
    State(subtitle_reader): State<SubtitleReader>,
    Path(id): Path<String>,
) -> Result<VttResponse, ApiError> {
    let id = VideoId::new(id)?;

    let vtt = run_blocking(move || subtitle_reader.read(&id))
        .await?
        .map_err(read_subtitles_error)?;
    Ok(vtt_response(vtt))
}

pub async fn list_video_categories(
    State(video_category_choice_searcher): State<VideoCategoryChoiceSearcher>,
    Path(id): Path<String>,
) -> Result<Json<VideoCategoriesResponse>, ApiError> {
    let id = VideoId::new(id)?;

    let choices = run_blocking(move || video_category_choice_searcher.search(&id))
        .await?
        .map_err(edit_video_categories_error)?;
    Ok(Json(VideoCategoriesResponse::from(choices)))
}

pub async fn set_video_category(
    State(video_category_editor): State<VideoCategoryEditor>,
    State(video_category_choice_searcher): State<VideoCategoryChoiceSearcher>,
    Path((id, category_id)): Path<(String, String)>,
    Json(request): Json<SetVideoCategoryRequest>,
) -> Result<Json<VideoCategoriesResponse>, ApiError> {
    let id = VideoId::new(id)?;
    let category_id = CategoryId::new(category_id)?;
    let belongs = required(
        request
            .belongs
            .as_ref()
            .and_then(serde_json::Value::as_bool),
        INVALID_BELONGS,
    )?;

    let choices = run_blocking(move || {
        video_category_editor.set(&id, &category_id, belongs)?;
        video_category_choice_searcher.search(&id)
    })
    .await?
    .map_err(edit_video_categories_error)?;
    Ok(Json(VideoCategoriesResponse::from(choices)))
}

pub async fn delete_video_ai_verdicts(
    State(video_category_editor): State<VideoCategoryEditor>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = VideoId::new(id)?;

    run_blocking(move || video_category_editor.rerun_video(&id))
        .await?
        .map_err(edit_video_categories_error)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn retry_video(
    State(video_retrier): State<VideoRetrier>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = VideoId::new(id)?;

    run_blocking(move || video_retrier.retry(&id))
        .await?
        .map_err(|error| match error {
            e @ RetryVideoError::VideoNotFound(_) => ApiError::new(StatusCode::NOT_FOUND, e),
            e @ RetryVideoError::Repository(_) => ApiError::internal(e),
        })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Maps a failed read or edit of a video's categories, the same for every
/// route touching them.
pub fn edit_video_categories_error(error: EditVideoCategoriesError) -> ApiError {
    match error {
        e @ (EditVideoCategoriesError::VideoNotFound
        | EditVideoCategoriesError::CategoryNotFound
        | EditVideoCategoriesError::SourceNotFound) => ApiError::new(StatusCode::NOT_FOUND, e),
        e @ EditVideoCategoriesError::FromSource(_) => ApiError::new(StatusCode::CONFLICT, e),
        e @ EditVideoCategoriesError::Repository(_) => ApiError::internal(e),
    }
}

pub fn update_watch_state_error(error: UpdateWatchStateError) -> ApiError {
    match error {
        e @ UpdateWatchStateError::VideoNotFound(_) => ApiError::bad_request(e),
        e @ UpdateWatchStateError::VideoNotDownloaded(_) => ApiError::bad_request(e),
        e @ UpdateWatchStateError::ChannelNotFound(_) => ApiError::bad_request(e),
        e @ UpdateWatchStateError::CategoryNotFound(_) => ApiError::new(StatusCode::NOT_FOUND, e),
        e @ UpdateWatchStateError::PlaylistNotFound(_) => ApiError::bad_request(e),
        e @ UpdateWatchStateError::Repository(_) => ApiError::internal(e),
    }
}

fn list_videos_error(error: ListVideosError) -> ApiError {
    match error {
        e @ ListVideosError::PlaylistNotFound(_) => ApiError::bad_request(e),
        e @ ListVideosError::ChannelNotFound(_) => ApiError::bad_request(e),
        e @ ListVideosError::CategoryNotFound(_) => ApiError::new(StatusCode::NOT_FOUND, e),
        e @ ListVideosError::Repository(_) => ApiError::internal(e),
    }
}

fn read_subtitles_error(error: ReadSubtitlesError) -> ApiError {
    match error {
        e @ ReadSubtitlesError::VideoNotFound(_) => ApiError::new(StatusCode::NOT_FOUND, e),
        e @ ReadSubtitlesError::SubtitlesNotFound(_) => ApiError::new(StatusCode::NOT_FOUND, e),
        e @ ReadSubtitlesError::Repository(_) => ApiError::internal(e),
    }
}

fn vtt_response(vtt: WebVtt) -> VttResponse {
    ([(CONTENT_TYPE, VTT_CONTENT_TYPE)], vtt.as_str().to_string())
}

#[cfg(test)]
mod tests;
