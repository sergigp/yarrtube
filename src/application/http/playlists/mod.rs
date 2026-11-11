pub mod dto;

use super::blocking::run_blocking;
use super::error::{ApiError, move_show_error};
use super::validation::{MISSING_CATEGORIZATION, MISSING_QUALITY, MISSING_SUMMARIES, required};
use super::videos::{edit_video_categories_error, update_watch_state_error};
use crate::domain::category::CategoryMember;
use crate::domain::playlist::PlaylistId;
use crate::domain::playlist::{
    CreatePlaylistError, DeletePlaylistError, PlaylistPath, PlaylistSettingsChange,
    PreviewPlaylistError, UpdatePlaylistError,
};
use crate::domain::services::{
    CreatePlaylistOutcome, PlaylistCreator, PlaylistCreatorApi, PlaylistDeleter,
    PlaylistDeleterApi, PlaylistPreviewer, PlaylistPreviewerApi, PlaylistUpdater,
    PlaylistUpdaterApi, PlaylistVideoReconciler, PlaylistVideoReconcilerApi, PlaylistViewSearcher,
    PlaylistViewSearcherApi, ShowMover, ShowMoverApi, VideoCategoryEditor, VideoCategoryEditorApi,
    VideoWatchStateUpdater, VideoWatchStateUpdaterApi,
};
use crate::domain::shared::{Quality, ReconcileInterval};
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use dto::{
    CreatePlaylistRequest, MovePlaylistRequest, PlaylistListItemResponse, PlaylistPreviewResponse,
    PlaylistResponse, PreviewPlaylistQuery, UpdatePlaylistRequest,
};

const MISSING_PATH: &str = "Playlist path must not be empty";
const NOTHING_TO_UPDATE: &str = "Request must change exclude from home, hide unwatched count, the reconcile interval, summaries or categorization";

pub async fn create_playlist(
    State(playlist_creator): State<PlaylistCreator>,
    Json(request): Json<CreatePlaylistRequest>,
) -> Result<(StatusCode, Json<PlaylistResponse>), ApiError> {
    let id = PlaylistId::from_url_or_id(request.playlist)?;
    let path = PlaylistPath::new(required(request.path, MISSING_PATH)?)?;
    let quality = Quality::new(required(request.quality, MISSING_QUALITY)?)?;
    let exclude_from_home = request.exclude_from_home.unwrap_or(false);
    let reconcile_interval = request
        .reconcile_interval_minutes
        .map(ReconcileInterval::new)
        .transpose()?
        .unwrap_or_else(ReconcileInterval::default_interval);
    let summaries_enabled = required(request.summaries_enabled, MISSING_SUMMARIES)?;
    let categorization_enabled = required(request.categorization_enabled, MISSING_CATEGORIZATION)?;

    let outcome = run_blocking(move || {
        playlist_creator.create(
            id,
            path,
            quality,
            exclude_from_home,
            reconcile_interval,
            summaries_enabled,
            categorization_enabled,
        )
    })
    .await?;

    match outcome {
        Ok(CreatePlaylistOutcome::Created(playlist)) => {
            Ok((StatusCode::CREATED, Json(PlaylistResponse::from(playlist))))
        }
        Ok(CreatePlaylistOutcome::AlreadyExisted(playlist)) => {
            Ok((StatusCode::OK, Json(PlaylistResponse::from(playlist))))
        }
        Err(e @ CreatePlaylistError::YoutubePlaylistNotFound(_)) => Err(ApiError::bad_request(e)),
        Err(e @ CreatePlaylistError::PathAlreadyInUse(_)) => Err(ApiError::bad_request(e)),
        Err(e @ CreatePlaylistError::Lookup(_)) => Err(ApiError::new(StatusCode::BAD_GATEWAY, e)),
        Err(e @ CreatePlaylistError::SummariesUnavailable) => {
            Err(ApiError::new(StatusCode::CONFLICT, e))
        }
        Err(e @ CreatePlaylistError::CategorizationUnavailable) => {
            Err(ApiError::new(StatusCode::CONFLICT, e))
        }
        Err(e @ CreatePlaylistError::Repository(_)) => Err(ApiError::internal(e)),
    }
}

pub async fn preview_playlist(
    State(playlist_previewer): State<PlaylistPreviewer>,
    Query(query): Query<PreviewPlaylistQuery>,
) -> Result<Json<PlaylistPreviewResponse>, ApiError> {
    let id = PlaylistId::from_url_or_id(query.playlist.unwrap_or_default())?;

    match run_blocking(move || playlist_previewer.preview(id)).await? {
        Ok(preview) => Ok(Json(PlaylistPreviewResponse::from(preview))),
        Err(e @ PreviewPlaylistError::YoutubePlaylistNotFound(_)) => {
            Err(ApiError::new(StatusCode::NOT_FOUND, e))
        }
        Err(e @ PreviewPlaylistError::Lookup(_)) => Err(ApiError::new(StatusCode::BAD_GATEWAY, e)),
    }
}

pub async fn delete_playlist(
    State(playlist_deleter): State<PlaylistDeleter>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = PlaylistId::new(id)?;

    match run_blocking(move || playlist_deleter.delete(id)).await? {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(e @ DeletePlaylistError::NotFound(_)) => Err(ApiError::bad_request(e)),
        Err(e @ DeletePlaylistError::Reconciling(_)) => Err(ApiError::new(StatusCode::CONFLICT, e)),
        Err(e @ DeletePlaylistError::Repository(_)) => Err(ApiError::internal(e)),
    }
}

pub async fn delete_playlist_ai_verdicts(
    State(video_category_editor): State<VideoCategoryEditor>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let source = CategoryMember::Playlist(PlaylistId::new(id)?);

    run_blocking(move || video_category_editor.rerun_source(&source))
        .await?
        .map_err(edit_video_categories_error)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn update_playlist(
    State(playlist_updater): State<PlaylistUpdater>,
    Path(id): Path<String>,
    Json(request): Json<UpdatePlaylistRequest>,
) -> Result<Json<PlaylistResponse>, ApiError> {
    let id = PlaylistId::new(id)?;
    let change = PlaylistSettingsChange {
        exclude_from_home: request.exclude_from_home,
        hide_unwatched_count: request.hide_unwatched_count,
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

    match run_blocking(move || playlist_updater.update_settings(id, change)).await? {
        Ok(playlist) => Ok(Json(PlaylistResponse::from(playlist))),
        Err(e @ UpdatePlaylistError::NotFound(_)) => Err(ApiError::new(StatusCode::NOT_FOUND, e)),
        Err(e @ UpdatePlaylistError::SummariesUnavailable) => {
            Err(ApiError::new(StatusCode::CONFLICT, e))
        }
        Err(e @ UpdatePlaylistError::CategorizationUnavailable) => {
            Err(ApiError::new(StatusCode::CONFLICT, e))
        }
        Err(e @ UpdatePlaylistError::Repository(_)) => Err(ApiError::internal(e)),
    }
}

pub async fn reconcile_playlist(
    State(playlist_video_reconciler): State<PlaylistVideoReconciler>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = PlaylistId::new(id)?;

    run_blocking(move || playlist_video_reconciler.force_reconcile(id))
        .await?
        .map_err(ApiError::internal)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn move_playlist(
    State(show_mover): State<ShowMover>,
    Path(id): Path<String>,
    Json(request): Json<MovePlaylistRequest>,
) -> Result<Json<PlaylistResponse>, ApiError> {
    let id = PlaylistId::new(id)?;
    let path = PlaylistPath::new(required(request.path, MISSING_PATH)?)?;

    run_blocking(move || show_mover.move_playlist(id, path))
        .await?
        .map(|playlist| Json(PlaylistResponse::from(playlist)))
        .map_err(move_show_error)
}

pub async fn list_playlists(
    State(playlist_view_searcher): State<PlaylistViewSearcher>,
) -> Result<Json<Vec<PlaylistListItemResponse>>, ApiError> {
    let playlists = run_blocking(move || playlist_view_searcher.search_all())
        .await?
        .map_err(ApiError::internal)?;
    Ok(Json(
        playlists
            .into_iter()
            .map(PlaylistListItemResponse::from)
            .collect(),
    ))
}

pub async fn mark_playlist_watched(
    State(video_watch_state_updater): State<VideoWatchStateUpdater>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let playlist_id = PlaylistId::new(id)?;

    run_blocking(move || video_watch_state_updater.mark_playlist_watched(&playlist_id))
        .await?
        .map_err(update_watch_state_error)?;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests;
