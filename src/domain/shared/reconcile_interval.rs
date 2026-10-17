use super::errors::ValidationError;
use chrono::Duration;

pub const DEFAULT_RECONCILE_INTERVAL_MINUTES: u32 = 180;

/// How long after one recurring reconcile of a source finishes the next one
/// runs, in whole minutes (at least 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconcileInterval(u32);

impl ReconcileInterval {
    pub fn new(minutes: i64) -> Result<Self, ValidationError> {
        if minutes < 1 {
            return Err(ValidationError(format!(
                "Reconcile interval must be at least 1 minute (got {minutes})"
            )));
        }
        Ok(Self(minutes as u32))
    }

    pub fn default_interval() -> Self {
        Self(DEFAULT_RECONCILE_INTERVAL_MINUTES)
    }

    pub fn minutes(&self) -> u32 {
        self.0
    }

    pub fn duration(&self) -> Duration {
        Duration::minutes(i64::from(self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_accept_one_minute() {
        assert_eq!(ReconcileInterval::new(1), Ok(ReconcileInterval(1)));
    }
}
