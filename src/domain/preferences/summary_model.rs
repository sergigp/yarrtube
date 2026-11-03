use crate::domain::shared::ValidationError;

const MAX_LENGTH: usize = 200;

/// The name of the model that writes summaries, as the LLM server knows it
/// (e.g. `qwen3:4b`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryModel(String);

impl SummaryModel {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        match !value.trim().is_empty() && value.chars().count() <= MAX_LENGTH {
            true => Ok(Self(value)),
            false => Err(ValidationError(format!(
                "Summary model must be named (got \"{value}\")"
            ))),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for SummaryModel {
    fn default() -> Self {
        Self("qwen3:4b".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_accept_a_model_name() {
        assert_eq!(
            SummaryModel::new("gemma3:4b").map(|model| model.as_str().to_string()),
            Ok("gemma3:4b".to_string())
        );
    }

    #[test]
    fn it_should_accept_a_name_of_200_characters() {
        let name = "m".repeat(200);

        assert_eq!(
            SummaryModel::new(name.clone()).map(|model| model.as_str().to_string()),
            Ok(name)
        );
    }

    #[test]
    fn it_should_reject_an_empty_name() {
        assert_eq!(SummaryModel::new(""), Err(rejected("")));
    }

    #[test]
    fn it_should_reject_a_blank_name() {
        assert_eq!(SummaryModel::new("  "), Err(rejected("  ")));
    }

    #[test]
    fn it_should_reject_a_name_over_200_characters() {
        let name = "m".repeat(201);

        assert_eq!(SummaryModel::new(name.clone()), Err(rejected(&name)));
    }

    fn rejected(value: &str) -> ValidationError {
        ValidationError(format!("Summary model must be named (got \"{value}\")"))
    }
}
