use super::errors::ValidationError;
use chrono::Duration;

pub const DEFAULT_RECONCILE_INTERVAL_MINUTES: u32 = 180;
const MAX_RECONCILE_INTERVAL_MINUTES: i64 = 24 * 60;

/// How long after one recurring reconcile of a source finishes the next one
/// runs, in whole minutes, from 1 minute to 24 hours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconcileInterval(u32);

impl ReconcileInterval {
    pub fn new(minutes: i64) -> Result<Self, ValidationError> {
        if minutes < 1 {
            return Err(ValidationError(format!(
                "Reconcile interval must be at least 1 minute (got {minutes})"
            )));
        }
        if minutes > MAX_RECONCILE_INTERVAL_MINUTES {
            return Err(ValidationError(format!(
                "Reconcile interval must be at most {MAX_RECONCILE_INTERVAL_MINUTES} minutes (got {minutes})"
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

    #[test]
    fn it_should_reject_zero() {
        assert_eq!(
            ReconcileInterval::new(0),
            Err(ValidationError(
                "Reconcile interval must be at least 1 minute (got 0)".to_string()
            ))
        );
    }

    #[test]
    fn it_should_reject_a_negative_interval() {
        assert_eq!(
            ReconcileInterval::new(-5),
            Err(ValidationError(
                "Reconcile interval must be at least 1 minute (got -5)".to_string()
            ))
        );
    }

    #[test]
    fn it_should_accept_a_day() {
        assert_eq!(ReconcileInterval::new(1440), Ok(ReconcileInterval(1440)));
    }

    #[test]
    fn it_should_reject_more_than_a_day() {
        assert_eq!(
            ReconcileInterval::new(1441),
            Err(ValidationError(
                "Reconcile interval must be at most 1440 minutes (got 1441)".to_string()
            ))
        );
    }
}
