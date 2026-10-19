//! Test-only capture of `tracing` events, for asserting a log line that is
//! a behaviour's only observable side effect.

use std::cell::RefCell;
use std::sync::Once;

thread_local! {
    /// The messages captured on this thread while `captured_log_messages`
    /// runs; `None` outside it, so other tests' events are dropped.
    static CAPTURED: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

/// Runs `body` capturing the `message` field of each event it logs on this
/// thread in memory (printing nothing, so the no-log-output test rule still
/// holds) and returns the captured messages — the only way to assert a pure
/// logging side effect (e.g. a warn event) fires or not.
pub(crate) fn captured_log_messages(body: impl FnOnce()) -> Vec<String> {
    install_capture_subscriber();
    let outer = CAPTURED.with(|captured| captured.replace(Some(Vec::new())));
    body();
    CAPTURED
        .with(|captured| captured.replace(outer))
        .unwrap_or_default()
}

/// Installs, once per test process, a global subscriber interested in every
/// event, which keeps only the ones logged inside `captured_log_messages`.
/// A scoped (thread-local) subscriber is not enough: a test thread running
/// with no subscriber can cache a callsite's interest as "never" at any
/// time, and the event is then skipped even on a capturing thread. With one
/// global, always-interested subscriber no callsite is ever cached as
/// "never".
fn install_capture_subscriber() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        use tracing_subscriber::layer::SubscriberExt;

        tracing::subscriber::set_global_default(tracing_subscriber::registry().with(CaptureLayer))
            .expect("no other global tracing subscriber is set in tests");
    });
}

/// Appends the `message` field of every tracing event to the thread's
/// capture buffer while one is active; see `captured_log_messages`.
struct CaptureLayer;

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

        let _ = CAPTURED.try_with(|captured| {
            if captured.borrow().is_none() {
                return;
            }
            let mut message = String::new();
            event.record(&mut MessageVisitor(&mut message));
            if let Some(messages) = captured.borrow_mut().as_mut() {
                messages.push(message);
            }
        });
    }
}
