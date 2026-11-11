pub mod dto;

use super::blocking::run_blocking;
use super::error::ApiError;
use crate::domain::llm_server::OllamaUrl;
use crate::domain::preferences::{
    CategorizationModel, CategorizationPrompt, ContextLength, PreferencesChange, SummaryModel,
    SummaryPrompt, Temperature, Theme, UpdatePreferencesError,
};
use crate::domain::services::{
    PreferencesSearcher, PreferencesSearcherApi, PreferencesUpdater, PreferencesUpdaterApi,
};
use axum::Json;
use axum::extract::State;
use dto::{PreferencesResponse, UpdatePreferencesRequest};

pub async fn get_preferences(
    State(preferences_searcher): State<PreferencesSearcher>,
) -> Result<Json<PreferencesResponse>, ApiError> {
    let preferences = run_blocking(move || preferences_searcher.get())
        .await?
        .map_err(ApiError::internal)?;
    Ok(Json(PreferencesResponse::from(preferences)))
}

pub async fn update_preferences(
    State(preferences_updater): State<PreferencesUpdater>,
    Json(request): Json<UpdatePreferencesRequest>,
) -> Result<Json<PreferencesResponse>, ApiError> {
    let change = preferences_change(request)?;
    let updated = run_blocking(move || preferences_updater.update(change))
        .await?
        .map_err(update_preferences_error)?;
    Ok(Json(PreferencesResponse::from(updated)))
}

/// The request's fields as value objects, rejecting the first invalid one.
fn preferences_change(request: UpdatePreferencesRequest) -> Result<PreferencesChange, ApiError> {
    Ok(PreferencesChange {
        categories_enabled: request.categories_enabled,
        home_show_continue_watching: request.home_show_continue_watching,
        home_show_categories: request.home_show_categories,
        home_show_quick_watches: request.home_show_quick_watches,
        home_show_latest: request.home_show_latest,
        theme: request.theme.map(Theme::new).transpose()?,
        ai_enabled: request.ai_enabled,
        ollama_url: request.ollama_url.map(ollama_url).transpose()?,
        summaries_enabled: request.summaries_enabled,
        summaries_model: request.summaries_model.map(SummaryModel::new).transpose()?,
        summaries_context_length: request
            .summaries_context_length
            .map(ContextLength::new)
            .transpose()?,
        summaries_temperature: request
            .summaries_temperature
            .map(Temperature::new)
            .transpose()?,
        summaries_thinking: request.summaries_thinking,
        summaries_prompt: request
            .summaries_prompt
            .map(|prompt| prompt.map(SummaryPrompt::new).transpose())
            .transpose()?,
        categorization_enabled: request.categorization_enabled,
        categorization_model: request
            .categorization_model
            .map(CategorizationModel::new)
            .transpose()?,
        categorization_context_length: request
            .categorization_context_length
            .map(|length| length.map(ContextLength::new).transpose())
            .transpose()?,
        categorization_temperature: request
            .categorization_temperature
            .map(Temperature::new)
            .transpose()?,
        categorization_thinking: request.categorization_thinking,
        categorization_prompt: request
            .categorization_prompt
            .map(|prompt| prompt.map(CategorizationPrompt::new).transpose())
            .transpose()?,
    })
}

/// An empty URL unsets it.
fn ollama_url(value: String) -> Result<Option<OllamaUrl>, ApiError> {
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(OllamaUrl::new(value)?))
    }
}

fn update_preferences_error(error: UpdatePreferencesError) -> ApiError {
    match error {
        UpdatePreferencesError::Invalid(e) => ApiError::from(e),
        e @ UpdatePreferencesError::Repository(_) => ApiError::internal(e),
    }
}

#[cfg(test)]
mod tests;
