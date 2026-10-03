## Files

- `src/domain/services/plex_collection_reconciler.rs` — `reconcile_collection` gains the "existing but empty → recreate" branch; it is a reconciliation decision, so it lives in the domain service.
- `src/infrastructure/repositories/plex_collection_repository.rs` — `ensure_success` consumes the response to put Plex's body in the error; HTTP detail stays in the adapter.
- `src/application/tasks/reconcile_plex_collections_task.rs` — behaviour tests (the task is the application entry point).

## Types & Signatures

No trait changes. Recreate reuses `delete_collection` + `create_collection`.

```rust
// plex_collection_reconciler.rs
impl PlexCollectionReconciler {
    fn reconcile_collection(&self, section_id: &str, name: &str, desired: Vec<String>, collections: &HashMap<String, String>) -> anyhow::Result<()>;
    fn converge_members(&self, section_id: &str, name: &str, collection_rating_key: &str, desired: &[String]) -> anyhow::Result<()>; // + section_id
    fn recreate_collection(&self, section_id: &str, name: &str, collection_rating_key: &str, desired: &[String]) -> anyhow::Result<()>;
}

// plex_collection_repository.rs — takes ownership so the body can be read
fn ensure_success(path: &str, response: reqwest::blocking::Response) -> anyhow::Result<reqwest::blocking::Response>;
// error: "Plex request to {path} failed with status {status}: {body}"
// body trimmed; ": {body}" omitted when empty
```

## Call Stack

Existing empty collection, desired non-empty:

```
ReconcilePlexCollectionsTask::handle
└─ PlexCollectionReconciler::reconcile_all
   └─ reconcile_{playlist,channel}_section(section_id)
      └─ reconcile_collection(section_id, name, desired, collections)   // Some(key)
         └─ converge_members(section_id, name, key, &desired)
            ├─ repo.list_collection_items(key)            → []
            └─ recreate_collection(section_id, name, key, &desired)   // members empty && desired non-empty
               ├─ repo.delete_collection(key)
               └─ repo.create_collection(section_id, name, &desired)  // sets collectionSort=1
               info!(section, collection, members, "recreated empty Plex collection")
```

Failed request (any verb):

```
HttpPlexCollectionRepository::{get_json, delete, add_items, create_collection, set_alphabetical_sort}
└─ ensure_success(path, response) → Err("… failed with status 400 Bad Request: <body>")
```

Decision: recreate keys off "zero members", not off a 400 from `add_items` — deterministic, no error-string sniffing, and an empty collection has nothing to lose. If `create_collection` fails after the delete, the next pass sees no collection and creates it (existing path).

## Test Plan

1. Behaviour (`reconcile_plex_collections_task`):
   - `it_should_recreate_an_empty_collection_with_its_scanned_videos` — existing collection `c1` "Lofi beats" with no members, playlist has scanned `101`,`102` → `Ok(())`; `mutations() == ["delete:c1", "create:1:Lofi beats"]`; `collections() == [FakePlexCollection { rating_key: "collection:Lofi beats", title: "Lofi beats", member_rating_keys: ["101","102"] }]`.
   - `it_should_leave_an_empty_collection_alone_if_no_video_is_scanned` — existing empty `c1`, playlist has a downloaded video not scanned → `mutations() == []`, `c1` unchanged.
2. Infrastructure (`plex_collection_repository`):
   - `it_should_fail_with_the_plex_response_body_if_the_server_replies_with_an_error` — mock `PUT /library/collections/c1/items` → 400 with body `bad uri`; `add_items` errs with `"Plex request to /library/collections/c1/items failed with status 400 Bad Request: bad uri"`.
   - `it_should_fail_if_the_server_replies_with_an_error` — existing test, unchanged message (empty body → no `: ` suffix).
