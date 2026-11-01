use std::any::Any;

/// The message a panic was raised with, as caught by `catch_unwind`: the
/// `&str` or `String` passed to `panic!`, or a placeholder for any other
/// payload.
pub fn panic_message(payload: &(dyn Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("unknown panic")
}
