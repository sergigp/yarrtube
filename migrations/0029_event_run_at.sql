-- When a pending domain event may next be dispatched: its publish time,
-- or after a failure, once its retry delay has passed. See the
-- domain-events capability.
ALTER TABLE events ADD COLUMN run_at TEXT NOT NULL DEFAULT '';
-- Events pending before this migration stay due immediately.
UPDATE events SET run_at = created_at;
