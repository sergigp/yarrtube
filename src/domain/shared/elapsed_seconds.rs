use chrono::{DateTime, Utc};

/// The seconds from `from` to `to`, to the millisecond, for log fields.
pub fn elapsed_seconds(from: DateTime<Utc>, to: DateTime<Utc>) -> f64 {
    (to - from).num_milliseconds() as f64 / 1000.0
}
