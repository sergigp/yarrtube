use super::error::ApiError;

pub const MISSING_QUALITY: &str = "Quality must be one of \"high\", \"mid\", or \"low\" (missing)";
pub const MISSING_POSITION: &str = "Playback position must not be negative (missing)";

/// Unwraps a field the request must carry, rejecting the request with a
/// `400 Bad Request` whose message is `missing_message` when it is absent.
pub fn required<T>(value: Option<T>, missing_message: &str) -> Result<T, ApiError> {
    value.ok_or_else(|| ApiError::bad_request(missing_message))
}
