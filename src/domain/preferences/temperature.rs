use crate::domain::shared::ValidationError;

/// How random the model's output is: 0 always picks the likeliest word.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Temperature(f64);

impl Temperature {
    pub fn new(value: f64) -> Result<Self, ValidationError> {
        match (0.0..=2.0).contains(&value) {
            true => Ok(Self(value)),
            false => Err(ValidationError(format!(
                "Temperature must be between 0 and 2 (got {value})"
            ))),
        }
    }

    /// What `evals/categories` runs with.
    pub fn categorization_default() -> Self {
        Self(0.0)
    }

    pub fn value(&self) -> f64 {
        self.0
    }
}

impl Default for Temperature {
    fn default() -> Self {
        Self(0.3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_accept_zero() {
        assert_eq!(Temperature::new(0.0).map(|t| t.value()), Ok(0.0));
    }

    #[test]
    fn it_should_accept_two() {
        assert_eq!(Temperature::new(2.0).map(|t| t.value()), Ok(2.0));
    }

    #[test]
    fn it_should_reject_a_negative_temperature() {
        assert_eq!(
            Temperature::new(-0.1),
            Err(ValidationError(
                "Temperature must be between 0 and 2 (got -0.1)".to_string()
            ))
        );
    }

    #[test]
    fn it_should_reject_above_two() {
        assert_eq!(
            Temperature::new(2.1),
            Err(ValidationError(
                "Temperature must be between 0 and 2 (got 2.1)".to_string()
            ))
        );
    }

    #[test]
    fn it_should_reject_not_a_number() {
        assert_eq!(
            Temperature::new(f64::NAN),
            Err(ValidationError(
                "Temperature must be between 0 and 2 (got NaN)".to_string()
            ))
        );
    }
}
