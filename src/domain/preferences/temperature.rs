use crate::domain::shared::ValidationError;

/// How random the model's output is: 0 always picks the likeliest word.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Temperature(f64);

impl Temperature {
    pub fn new(value: f64) -> Result<Self, ValidationError> {
        Ok(Self(value))
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
