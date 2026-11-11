pub mod dto;

use super::blocking::run_blocking;
use super::error::{ApiError, move_show_error};
use super::validation::{MISSING_CATEGORIZATION, MISSING_QUALITY, MISSING_SUMMARIES, required};
use super::videos::{edit_video_categories_error, update_watch_state_error};
use crate::domain::category::CategoryMember;
use crate::domain::channel::{
    ChannelHandle, ChannelSettingsChange, CreateChannelError, DeleteChannelError,
    PreviewChannelError, UpdateChannelError, VideoLimit,
};
use crate::domain::playlist::PlaylistPath;
use crate::domain::services::{
    ChannelCreator, ChannelCreatorApi, ChannelDeleter, ChannelDeleterApi, ChannelPreviewer,
    ChannelPreviewerApi, ChannelUpdater, ChannelUpdaterApi, ChannelVideoReconciler,
    ChannelVideoReconcilerApi, ChannelViewSearcher, ChannelViewSearcherApi, CreateChannelOutcome,
    ShowMover, ShowMoverApi, VideoCategoryEditor, VideoCategoryEditorApi, VideoWatchStateUpdater,
    VideoWatchStateUpdaterApi,
};
use crate::domain::shared::{Quality, ReconcileInterval};
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use dto::{
    ChannelListItemResponse, ChannelPreviewResponse, ChannelResponse, CreateChannelRequest,
    MoveChannelRequest, PreviewChannelQuery, UpdateChannelRequest,
};

const MISSING_VIDEO_LIMIT: &str = "Video limit must be a positive integer (missing)";
const MISSING_PATH: &str = "Channel path must not be empty";
const NOTHING_TO_UPDATE: &str = "Request must change the quality, the video limit, the reconcile interval, summaries or categorization";

pub async fn create_channel(
    State(channel_creator): State<ChannelCreator>,
    Json(request): Json<CreateChannelRequest>,
) -> Result<(StatusCode, Json<ChannelResponse>), ApiError> {
    let id = ChannelHandle::from_url_or_handle(request.channel.unwrap_or_default())?;
    let quality = Quality::new(required(request.quality, MISSING_QUALITY)?)?;
    let video_limit = VideoLimit::new(required(request.video_limit, MISSING_VIDEO_LIMIT)?)?;
    let path = PlaylistPath::new(required(request.path, MISSING_PATH)?)?;
    let reconcile_interval = request
        .reconcile_interval_minutes
        .map(ReconcileInterval::new)
        .transpose()?
        .unwrap_or_else(ReconcileInterval::default_interval);
    let summaries_enabled = required(request.summaries_enabled, MISSING_SUMMARIES)?;
    let categorization_enabled = required(request.categorization_enabled, MISSING_CATEGORIZATION)?;

    let outcome = run_blocking(move || {
        channel_creator.create(
            id,
            quality,
            video_limit,
            reconcile_interval,
            path,
            summaries_enabled,
            categorization_enabled,
        )
    })
    .await?;

    match outcome {
        Ok(CreateChannelOutcome::Created(channel)) => {
            Ok((StatusCode::CREATED, Json(ChannelResponse::from(channel))))
        }
        Ok(CreateChannelOutcome::AlreadyExisted(channel)) => {
            Ok((StatusCode::OK, Json(ChannelResponse::from(channel))))
        }
        Err(e @ CreateChannelError::YoutubeChannelNotFound(_)) => Err(ApiError::bad_request(e)),
        Err(e @ CreateChannelError::Lookup(_)) => Err(ApiError::new(StatusCode::BAD_GATEWAY, e)),
        Err(e @ CreateChannelError::SummariesUnavailable) => {
            Err(ApiError::new(StatusCode::CONFLICT, e))
        }
        Err(e @ CreateChannelError::CategorizationUnavailable) => {
            Err(ApiError::new(StatusCode::CONFLICT, e))
        }
        Err(e @ CreateChannelError::Repository(_)) => Err(ApiError::internal(e)),
    }
}

pub async fn preview_channel(
    State(channel_previewer): State<ChannelPreviewer>,
    Query(query): Query<PreviewChannelQuery>,
) -> Result<Json<ChannelPreviewResponse>, ApiError> {
    let id = ChannelHandle::from_url_or_handle(query.channel.unwrap_or_default())?;

    match run_blocking(move || channel_previewer.preview(id)).await? {
        Ok(preview) => Ok(Json(ChannelPreviewResponse::from(preview))),
        Err(e @ PreviewChannelError::YoutubeChannelNotFound(_)) => {
            Err(ApiError::new(StatusCode::NOT_FOUND, e))
        }
        Err(e @ PreviewChannelError::Lookup(_)) => Err(ApiError::new(StatusCode::BAD_GATEWAY, e)),
    }
}

pub async fn delete_channel(
    State(channel_deleter): State<ChannelDeleter>,
    Path(handle): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = ChannelHandle::new(handle)?;

    match run_blocking(move || channel_deleter.delete(id)).await? {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(e @ DeleteChannelError::NotFound(_)) => Err(ApiError::bad_request(e)),
        Err(e @ DeleteChannelError::Reconciling(_)) => Err(ApiError::new(StatusCode::CONFLICT, e)),
        Err(e @ DeleteChannelError::Repository(_)) => Err(ApiError::internal(e)),
    }
}

pub async fn delete_channel_ai_verdicts(
    State(video_category_editor): State<VideoCategoryEditor>,
    Path(handle): Path<String>,
) -> Result<StatusCode, ApiError> {
    let source = CategoryMember::Channel(ChannelHandle::new(handle)?);

    run_blocking(move || video_category_editor.rerun_source(&source))
        .await?
        .map_err(edit_video_categories_error)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn update_channel(
    State(channel_updater): State<ChannelUpdater>,
    Path(handle): Path<String>,
    Json(request): Json<UpdateChannelRequest>,
) -> Result<Json<ChannelResponse>, ApiError> {
    let id = ChannelHandle::new(handle)?;
    let change = ChannelSettingsChange {
        quality: request.quality.map(Quality::new).transpose()?,
        video_limit: request.video_limit.map(VideoLimit::new).transpose()?,
        reconcile_interval: request
            .reconcile_interval_minutes
            .map(ReconcileInterval::new)
            .transpose()?,
        summaries_enabled: request.summaries_enabled,
        categorization_enabled: request.categorization_enabled,
    };
    if change.is_empty() {
        return Err(ApiError::bad_request(NOTHING_TO_UPDATE));
    }

    match run_blocking(move || channel_updater.update_settings(id, change)).await? {
        Ok(channel) => Ok(Json(ChannelResponse::from(channel))),
        Err(e @ UpdateChannelError::NotFound(_)) => Err(ApiError::new(StatusCode::NOT_FOUND, e)),
        Err(e @ UpdateChannelError::SummariesUnavailable) => {
            Err(ApiError::new(StatusCode::CONFLICT, e))
        }
        Err(e @ UpdateChannelError::CategorizationUnavailable) => {
            Err(ApiError::new(StatusCode::CONFLICT, e))
        }
        Err(e @ UpdateChannelError::Repository(_)) => Err(ApiError::internal(e)),
    }
}

pub async fn move_channel(
    State(show_mover): State<ShowMover>,
    Path(handle): Path<String>,
    Json(request): Json<MoveChannelRequest>,
) -> Result<Json<ChannelResponse>, ApiError> {
    let id = ChannelHandle::new(handle)?;
    let path = PlaylistPath::new(required(request.path, MISSING_PATH)?)?;

    run_blocking(move || show_mover.move_channel(id, path))
        .await?
        .map(|channel| Json(ChannelResponse::from(channel)))
        .map_err(move_show_error)
}

pub async fn list_channels(
    State(channel_view_searcher): State<ChannelViewSearcher>,
) -> Result<Json<Vec<ChannelListItemResponse>>, ApiError> {
    let channels = run_blocking(move || channel_view_searcher.search_all())
        .await?
        .map_err(ApiError::internal)?;
    Ok(Json(
        channels
            .into_iter()
            .map(ChannelListItemResponse::from)
            .collect(),
    ))
}

pub async fn reconcile_channel(
    State(channel_video_reconciler): State<ChannelVideoReconciler>,
    Path(handle): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = ChannelHandle::new(handle)?;

    run_blocking(move || channel_video_reconciler.force_reconcile(id))
        .await?
        .map_err(ApiError::internal)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn mark_channel_watched(
    State(video_watch_state_updater): State<VideoWatchStateUpdater>,
    Path(handle): Path<String>,
) -> Result<StatusCode, ApiError> {
    let channel_id = ChannelHandle::new(handle)?;

    run_blocking(move || video_watch_state_updater.mark_channel_watched(&channel_id))
        .await?
        .map_err(update_watch_state_error)?;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests;
