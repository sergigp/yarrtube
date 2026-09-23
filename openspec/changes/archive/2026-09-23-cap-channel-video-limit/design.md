## Context

`VideoLimit` is the channel aggregate's value object for the video limit. Its constructor takes the raw `i64` the HTTP request (`CreateChannelRequest.video_limit`) and the SQLite row (`channels.video_limit INTEGER`) both carry. It rejects values `<= 0` and then narrows with `value as u32`, which wraps silently above `u32::MAX` (see proposal.md, Why). All value objects return the shared `ValidationError`, which the HTTP layer maps to 400 with the VO's message, so the handler needs no change.

The same constructor runs on both write and read. `SqliteChannelRepository::row_to_channel` rebuilds each stored channel with `VideoLimit::new(row.video_limit)`, and a failure there fails the whole `list()`.

The web UI's add dialog posts `video_limit: Number(input)` from an `<input type="number" min="1" step="1" required>` inside a `<form>` that relies on native browser constraint validation (no `noValidate`).

## Goals / Non-Goals

**Goals:**
- One range rule, 1..=1000, owned by the `VideoLimit` value object and enforced on every construction, whether from a request or from a stored row.
- A lossless conversion from the validated `i64` to the stored `u32`: no wrapping cast remains.

**Non-Goals:**
- Making the channel listing tolerate unreadable rows. One bad row still fails `list()`; this change only makes such rows impossible to create through the API.
- A data migration (see Decisions).
- Changing the missing-value message (`Video limit must be a positive integer (missing)`), which comes from the handler's `required(...)` check, not from the VO.

## Decisions

**Enforce the range in `VideoLimit::new`, the single constructor.**
Check `(1..=MAX_VIDEO_LIMIT).contains(&value)` with `const MAX_VIDEO_LIMIT: i64 = 1000`, and convert the validated value with `u32::try_from` (infallible after the range check) instead of `as`.
- *Alternative: a separate unchecked constructor for rows read from storage.* Rejected. It creates two rules for one value, and the whole point is that a stored `VideoLimit` always satisfies the invariant. It is also unnecessary here (see the next decision).
- *Alternative: clamp values above 1000 to 1000.* Rejected. It would silently store a limit the caller did not ask for, the same class of surprise as the wrapping bug.

**No migration for existing rows.**
The capped constructor also runs on read, so a stored limit above 1000 would stop being readable. The only deployment has no stored limit above 10 and no rows corrupted by the wrap (confirmed by the owner), so no stored row can violate the new rule.
- *Alternative: a migration clamping `video_limit > 1000` to 1000 and repairing `<= 0` rows.* Not needed for the known data. Revisit only if the app gains other deployments before this ships.

**One message for every out-of-range value.**
`Video limit must be between 1 and 1000 (got N)` for `0`, negative values, and values above 1000 alike. This replaces `Video limit must be a positive integer (got N)` for `<= 0`, so the message always states the full accepted range.

**UI: add `max="1000"` to the existing number input.**
Native constraint validation already blocks submission for `min`/`step`/`required` violations and marks the field invalid. `max` joins the same mechanism, so there is no custom validation code. The API remains the authority.

## Risks / Trade-offs

- [A stored row above 1000 would make `GET /channels` fail] → No such row exists in the only deployment. Any new value is capped by the same constructor, so no new ones can appear.
- [The UI's `max` only covers the dialog; other clients can still send out-of-range values] → The API rejects them with a 400 and the range message.
- [The message wording for `<= 0` changes] → Only observable as the 400 body text. The web UI displays it as-is, and there are no other known consumers.
