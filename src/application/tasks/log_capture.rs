//! Test-only capture of `tracing` events, for asserting a log line that is
//! a behaviour's only observable side effect.

use std::sync::Arc;

/// Runs `body` with a scoped tracing subscriber that captures each event's
/// `message` field in memory (printing nothing, so the no-log-output test
/// rule still holds) and returns the captured messages — the only way to
/// assert a pure logging side effect (e.g. a warn event) fires or not.
pub(crate) fn captured_log_messages(body: impl FnOnce()) -> Vec<String> {
    use tracing_subscriber::layer::SubscriberExt;

    let messages: Arc<std::sync::Mutex<Vec<String>>> = Arc::new(std::sync::Mutex::new(vec![]));
    let subscriber = tracing_subscriber::registry().with(CaptureLayer(Arc::clone(&messages)));
    tracing::subscriber::with_default(subscriber, body);
    messages.lock().unwrap().clone()
}

/// Captures the `message` field of every tracing event into a shared
/// buffer; see `captured_log_messages`.
struct CaptureLayer(Arc<std::sync::Mutex<Vec<String>>>);

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for CaptureLayer {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        struct MessageVisitor<'a>(&'a mut String);
        impl tracing::field::Visit for MessageVisitor<'_> {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                if field.name() == "message" {
                    use std::fmt::Write;
                    let _ = write!(self.0, "{value:?}");
                }
            }
        }

        let mut message = String::new();
        event.record(&mut MessageVisitor(&mut message));
        self.0.lock().unwrap().push(message);
    }
}
