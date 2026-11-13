use super::*;
use crate::domain::llm_server::{LlmModel, LlmServer};
use crate::infrastructure::repositories::llm_server_repository::FakeLlmServerRepository;
use dto::{AnthropicModelResponse, OllamaModelResponse};
use std::sync::Arc;

#[tokio::test]
async fn it_should_list_ollama_models_at_the_llm_path() {
    let repository = Arc::new(FakeLlmServerRepository::new(LlmServer {
        version: Some("0.40.13".to_string()),
        models: vec![
            LlmModel {
                name: "qwen3:4b".to_string(),
                display_name: None,
                size_bytes: Some(2_497_293_918),
                thinking: true,
                context_window: None,
            },
            LlmModel {
                name: "gemma3:4b".to_string(),
                display_name: None,
                size_bytes: Some(3_338_801_804),
                thinking: false,
                context_window: None,
            },
        ],
    }));
    let llm_server_searcher = LlmServerSearcher::new(repository, false);
    let query = ollama_query();

    let response = get(llm_server_searcher, query).await;

    assert_eq!(
        response,
        Ok(OllamaResponse {
            version: "0.40.13".to_string(),
            models: vec![
                OllamaModelResponse {
                    name: "gemma3:4b".to_string(),
                    size_bytes: 3_338_801_804,
                    thinking: false,
                },
                OllamaModelResponse {
                    name: "qwen3:4b".to_string(),
                    size_bytes: 2_497_293_918,
                    thinking: true,
                },
            ],
        })
    );
}

#[tokio::test]
async fn it_should_return_no_models_if_none_installed() {
    let repository = Arc::new(FakeLlmServerRepository::new(LlmServer {
        version: Some("0.40.13".to_string()),
        models: vec![],
    }));
    let llm_server_searcher = LlmServerSearcher::new(repository, false);
    let query = ollama_query();

    let response = get(llm_server_searcher, query).await;

    assert_eq!(
        response,
        Ok(OllamaResponse {
            version: "0.40.13".to_string(),
            models: vec![],
        })
    );
}

#[tokio::test]
async fn it_should_fail_if_the_url_is_missing() {
    let response = get(any_llm_server_searcher(), OllamaQuery::default()).await;

    assert_eq!(
        response,
        Err(ApiError::bad_request("Ollama URL must be stated (missing)"))
    );
}

#[tokio::test]
async fn it_should_fail_if_the_url_is_not_http() {
    let query = OllamaQuery {
        url: Some("mac.local:11434".to_string()),
    };

    let response = get(any_llm_server_searcher(), query).await;

    assert_eq!(
        response,
        Err(ApiError::bad_request(
            "Ollama URL must be an http(s) URL (got \"mac.local:11434\")"
        ))
    );
}

#[tokio::test]
async fn it_should_fail_if_ollama_is_unreachable() {
    let llm_server_searcher =
        LlmServerSearcher::new(Arc::new(FakeLlmServerRepository::failing()), false);
    let query = ollama_query();

    let response = get(llm_server_searcher, query).await;

    assert_eq!(
        response,
        Err(ApiError::new(
            StatusCode::BAD_GATEWAY,
            "Could not reach Ollama at http://mac.local:11434: connection refused"
        ))
    );
}

#[tokio::test]
async fn it_should_list_anthropic_models_in_the_listed_order() {
    let repository = Arc::new(FakeLlmServerRepository::new(LlmServer {
        version: None,
        models: vec![
            anthropic_model("claude-sonnet-5-5", "Claude Sonnet 5.5"),
            anthropic_model("claude-haiku-5-5", "Claude Haiku 5.5"),
        ],
    }));
    let llm_server_searcher = LlmServerSearcher::new(repository, true);

    let response = anthropic(llm_server_searcher).await;

    assert_eq!(
        response,
        Ok(AnthropicResponse {
            models: vec![
                AnthropicModelResponse {
                    name: "claude-sonnet-5-5".to_string(),
                    display_name: "Claude Sonnet 5.5".to_string(),
                    context_window: 1_000_000,
                },
                AnthropicModelResponse {
                    name: "claude-haiku-5-5".to_string(),
                    display_name: "Claude Haiku 5.5".to_string(),
                    context_window: 1_000_000,
                },
            ],
        })
    );
}

#[tokio::test]
async fn it_should_answer_404_when_anthropic_isnt_configured() {
    let llm_server_searcher = LlmServerSearcher::new(
        Arc::new(FakeLlmServerRepository::new(LlmServer {
            version: None,
            models: vec![anthropic_model("claude-haiku-5-5", "Claude Haiku 5.5")],
        })),
        false,
    );

    let response = anthropic(llm_server_searcher).await;

    assert_eq!(
        response,
        Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "Anthropic isn't configured: set YARRTUBE_ANTHROPIC_API_KEY"
        ))
    );
}

#[tokio::test]
async fn it_should_answer_502_when_anthropic_cant_be_reached() {
    let llm_server_searcher =
        LlmServerSearcher::new(Arc::new(FakeLlmServerRepository::failing()), true);

    let response = anthropic(llm_server_searcher).await;

    assert_eq!(
        response,
        Err(ApiError::new(
            StatusCode::BAD_GATEWAY,
            "Could not reach Anthropic: connection refused"
        ))
    );
}

async fn get(
    llm_server_searcher: LlmServerSearcher,
    query: OllamaQuery,
) -> Result<OllamaResponse, ApiError> {
    get_ollama(State(llm_server_searcher), Query(query))
        .await
        .map(|Json(response)| response)
}

/// A searcher for tests whose request is rejected before reaching it: its
/// server is down, so a request that wrongly got through would fail with
/// another error instead of passing.
fn any_llm_server_searcher() -> LlmServerSearcher {
    LlmServerSearcher::new(Arc::new(FakeLlmServerRepository::failing()), false)
}

fn ollama_query() -> OllamaQuery {
    OllamaQuery {
        url: Some("http://mac.local:11434".to_string()),
    }
}

async fn anthropic(llm_server_searcher: LlmServerSearcher) -> Result<AnthropicResponse, ApiError> {
    get_anthropic(State(llm_server_searcher))
        .await
        .map(|Json(response)| response)
}

fn anthropic_model(name: &str, display_name: &str) -> LlmModel {
    LlmModel {
        name: name.to_string(),
        display_name: Some(display_name.to_string()),
        size_bytes: None,
        thinking: false,
        context_window: Some(1_000_000),
    }
}
