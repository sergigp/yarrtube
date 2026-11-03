use crate::domain::shared::ValidationError;

const MIN: u32 = 1024;
const MAX: u32 = 131072;

/// How many tokens the model reads at once (Ollama's `num_ctx`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextLength(u32);

impl ContextLength {
    pub fn new(value: i64) -> Result<Self, ValidationError> {
        match u32::try_from(value) {
            Ok(length) if (MIN..=MAX).contains(&length) => Ok(Self(length)),
            _ => Err(ValidationError(format!(
                "Context length must be between {MIN} and {MAX} (got {value})"
            ))),
        }
    }

    pub fn value(&self) -> u32 {
        self.0
    }
}

impl Default for ContextLength {
    fn default() -> Self {
        Self(12288)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_accept_the_minimum() {
        assert_eq!(ContextLength::new(1024).map(|c| c.value()), Ok(1024));
    }

    #[test]
    fn it_should_accept_the_maximum() {
        assert_eq!(ContextLength::new(131072).map(|c| c.value()), Ok(131072));
    }

    #[test]
    fn it_should_reject_below_the_minimum() {
        assert_eq!(ContextLength::new(1023), Err(rejected(1023)));
    }

    #[test]
    fn it_should_reject_above_the_maximum() {
        assert_eq!(ContextLength::new(131073), Err(rejected(131073)));
    }

    #[test]
    fn it_should_reject_a_negative_length() {
        assert_eq!(ContextLength::new(-1), Err(rejected(-1)));
    }

    fn rejected(value: i64) -> ValidationError {
        ValidationError(format!(
            "Context length must be between 1024 and 131072 (got {value})"
        ))
    }
}
