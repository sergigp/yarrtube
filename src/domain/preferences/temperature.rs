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

    pub fn value(&self) -> f64 {
        self.0
    }
}

impl Default for Temperature {
    fn default() -> Self {
        Self(0.3)
    }
}
