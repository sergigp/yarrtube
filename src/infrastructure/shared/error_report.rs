//! A single way to render an error for a log line, so every failure we log
//! names *why* it happened and not only the outermost "what".
//!
//! `error = %e` (plain `Display`) on an `anyhow::Error` prints only the
//! top-most context — "YouTube API request failed", "error sending request
//! for url (…)" — and hides the real cause sitting in its `source()` chain
//! (a refused connection, a timeout, an HTTP error body). These WARN/ERROR
//! lines are grepped every morning and fed to an LLM to hunt bugs, so the
//! cause is exactly the signal that must survive. [`cause_chain`] walks the
//! whole chain onto one line (`outermost: cause: root cause`, the same shape
//! as `anyhow`'s `{:#}`) and bounds its length so a pathological error body
//! can't blow the line up.

/// Upper bound on a rendered chain, in bytes. Generous enough to keep a full
/// causal chain plus a short HTTP body, small enough that a multi-kilobyte
/// error payload can't dominate a log line (or the `last_error` column).
const MAX_LEN: usize = 800;

/// Renders `error` and its full `source()` chain as a single line, truncated
/// to [`MAX_LEN`] bytes on a UTF-8 boundary with a trailing marker when it
/// overflows.
pub fn cause_chain(error: &anyhow::Error) -> String {
    truncate(format!("{error:#}"))
}

fn truncate(mut rendered: String) -> String {
    if rendered.len() <= MAX_LEN {
        return rendered;
    }
    let boundary = rendered
        .char_indices()
        .map(|(i, _)| i)
        .take_while(|i| *i <= MAX_LEN)
        .last()
        .unwrap_or(0);
    rendered.truncate(boundary);
    rendered.push_str("… (truncated)");
    rendered
}

#[cfg(test)]
mod tests {
    use super::cause_chain;
    use anyhow::anyhow;

    #[test]
    fn it_should_join_the_full_source_chain_onto_one_line() {
        let error = anyhow!("Connection refused (os error 111)")
            .context("error sending request for url (https://example/api)")
            .context("YouTube API request failed");

        let rendered = cause_chain(&error);

        assert_eq!(
            rendered,
            "YouTube API request failed: error sending request for url \
             (https://example/api): Connection refused (os error 111)"
        );
    }

    #[test]
    fn it_should_render_a_single_context_error_as_just_its_message() {
        let error = anyhow!("handler failed");

        let rendered = cause_chain(&error);

        assert_eq!(rendered, "handler failed");
    }

    #[test]
    fn it_should_truncate_an_overlong_chain_on_a_utf8_boundary() {
        let error = anyhow!("é".repeat(2000)).context("request failed");

        let rendered = cause_chain(&error);

        assert!(rendered.ends_with("… (truncated)"));
        assert!(rendered.len() <= 800 + "… (truncated)".len() + 1);
        assert!(rendered.starts_with("request failed: "));
    }
}
