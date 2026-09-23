## Files

| File | Why |
| --- | --- |
| `src/domain/directory/mod.rs` | New aggregate module; re-exports `Directory`, `DirectoryPath`, errors. |
| `src/domain/directory/directory.rs` | `Directory` entity: a listed directory and its immediate subdirectory names. |
| `src/domain/directory/directory_path.rs` | `DirectoryPath` VO: a path relative to the videos root, empty meaning the root itself. Separate from `PlaylistPath`, which forbids the empty path a browse of the root needs. |
| `src/domain/directory/errors.rs` | `DirectoryPathError` and `ListDirectoriesError` for this aggregate. |
| `src/domain/services/directory_searcher.rs` | `DirectorySearcher` use case: the only caller of the directory port. |
| `src/domain/services/mod.rs` | Register and re-export `DirectorySearcher`. |
| `src/domain/mod.rs` | Register the `directory` module. |
| `src/infrastructure/repositories/filesystem_directory_repository.rs` | `DirectoryRepository` port + `FilesystemDirectoryRepository`, which owns the videos root and enforces confinement. |
| `src/infrastructure/repositories/mod.rs` | Register the new repository module. |
| `src/application/http/directories/mod.rs` | `list_directories` handler: validates the query into `DirectoryPath`, maps outcomes to status codes. |
| `src/application/http/directories/dto.rs` | Wire types for the listing response + `From<Directory>`. |
| `src/application/http/mod.rs` | `GET /directories` route; `directory_searcher` and `videos_root` on `AppState`. The root is carried in the adapter layer, as the subscribers already do, so no domain type has to know a filesystem location. |
| `src/serve.rs` | Wire the repository and searcher; add `run_startup_storage_directories_check`. |
| `web/src/api.js` | `fetchDirectories(path)` client call. |
| `web/src/components/LocationField.jsx` | New: parent browser (breadcrumb + listing + create-folder), folder name input, destination preview. Owns the staged-parent state, which has no server representation. |
| `web/src/components/AddDialog.jsx` | Use `LocationField`; drop the free-text path and the conflict auto-expand. |
| `smoke-tests/helpers/addDialog.js` | Drive the new location controls instead of the path field. |
| `smoke-tests/tests/playlist.spec.js`, `smoke-tests/tests/channel.spec.js` | Follow the helper's new surface. |

## Types & Signatures

```rust
// domain/directory/directory_path.rs
pub struct DirectoryPath(String);

impl DirectoryPath {
    pub fn new(path: impl Into<String>) -> Result<Self, DirectoryPathError>;
    pub fn root() -> Self;
    pub fn as_str(&self) -> &str;
    pub fn is_root(&self) -> bool;
}

impl fmt::Display for DirectoryPath {}
```

```rust
// domain/directory/directory.rs
pub struct Directory {
    pub path: DirectoryPath,
    pub subdirectories: Vec<String>,
}
```

```rust
// domain/directory/errors.rs
pub struct DirectoryPathError(pub String);

pub enum ListDirectoriesError {
    NotFound(DirectoryPath),
    Repository(anyhow::Error),
}
```

```rust
// infrastructure/repositories/filesystem_directory_repository.rs
pub trait DirectoryRepository: Send + Sync {
    /// `Ok(None)` when nothing listable is at `path`: missing, not a
    /// directory, or resolving outside the videos root. Collapsing those
    /// three into one answer is deliberate — it keeps the handler from
    /// disclosing whether a path outside the root exists.
    fn list(&self, path: &DirectoryPath) -> anyhow::Result<Option<Directory>>;
}

pub struct FilesystemDirectoryRepository {
    videos_root: PathBuf,
}

impl FilesystemDirectoryRepository {
    pub fn new(videos_root: PathBuf) -> Self;
}
```

```rust
// domain/services/directory_searcher.rs
#[derive(Clone)]
pub struct DirectorySearcher {
    repository: Arc<dyn DirectoryRepository>,
}

impl DirectorySearcher {
    pub fn new(repository: Arc<dyn DirectoryRepository>) -> Self;
    pub fn list(&self, path: &DirectoryPath) -> Result<Directory, ListDirectoriesError>;
}
```

```rust
// application/http/directories/dto.rs
#[derive(Deserialize)]
pub struct ListDirectoriesQuery {
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Serialize)]
pub struct DirectoryResponse {
    /// Absolute configured videos root. No other endpoint exposes it, and
    /// the destination preview cannot be rendered without it.
    pub root: String,
    pub path: String,
    pub entries: Vec<DirectoryEntryResponse>,
}

#[derive(Serialize)]
pub struct DirectoryEntryResponse {
    pub name: String,
}

impl DirectoryResponse {
    pub fn new(videos_root: &str, directory: Directory) -> Self;
}
```

```rust
// application/http/directories/mod.rs
pub async fn list_directories(
    State(state): State<AppState>,
    Query(query): Query<ListDirectoriesQuery>,
) -> Response;
```

```rust
// serve.rs
const DEFAULT_STORAGE_DIRECTORIES: &[&str] = &["playlists", "channels"];

fn run_startup_storage_directories_check();
```

## Call Stack

**List directories — `GET /api/directories?path=playlists`**

```
list_directories(State(AppState), Query(ListDirectoriesQuery { path: Some("playlists") }))
  DirectoryPath::new("playlists")            // None -> DirectoryPath::root()
    Err(DirectoryPathError)                  -> 400 error_response(e.to_string())
  state.directory_searcher.list(&DirectoryPath)
    DirectoryRepository::list(&DirectoryPath)
      videos_root.join(path) -> canonicalize -> starts_with(canonicalized videos_root)
      read_dir -> keep dirs only, drop dot-prefixed -> sort by name
      Ok(Some(Directory { path, subdirectories }))
    Ok(None)                                 -> Err(ListDirectoriesError::NotFound(path))
  Ok(Directory)                              -> 200 Json(DirectoryResponse::new(&state.videos_root, directory))
  Err(NotFound)                              -> 404 error_response(e.to_string())
  Err(Repository)                            -> 500 error_response(e.to_string())
```

**Startup seeding — `serve()`**

```
serve()
  run_startup_storage_directories_check()
    for dir in DEFAULT_STORAGE_DIRECTORIES
      std::fs::create_dir_all(videos_path().join(dir))
        Ok(())  -> info!(dir)
        Err(e)  -> error!(dir, error = %e)   // continues starting up
  build_application()
    FilesystemDirectoryRepository::new(PathBuf::from(videos_path()))
    DirectorySearcher::new(Arc<dyn DirectoryRepository>)
    AppState { directory_searcher, videos_root: videos_path(), .. }
```

**Add dialog submit — `web/src/components/AddDialog.jsx`**

```
LocationField(mode, sourceName)
  fetchDirectories(parent)                     -> GET /api/directories?path=<parent>
    parent is staged (adopted, not on disk)    -> skip the request, entries = []
    { root, path, entries }                    -> root supplies the preview's absolute prefix
  fetchPlaylists() + fetchChannels()           -> occupied paths, keyed by path
  select an entry                              -> parent = `${parent}/${entry.name}`
  select a breadcrumb ancestor                 -> parent = that ancestor
  create folder(name)
    name already in entries                    -> descend into it, not staged
    otherwise                                  -> parent = `${parent}/${name}`, staged
  destination = `${root}/${parent}/${folderName}`
    newSegments = staged parent segments
               ++ (folderName in entries ? [] : [folderName])
    destination occupied by a playlist/channel -> report conflict, block submit
  onChange({ path: `${parent}/${folderName}` })   // relative; root is display-only
AddDialog.handleSubmit()
  createPlaylist({ playlist, name, path, quality })
  createChannel({ channel, quality, video_limit, path })
```

## Test Plan

**`domain/directory/directory_path.rs`** (VO validation, the one place with rules worth unit-testing)
- `it_should_accept_a_single_segment_path`
- `it_should_accept_a_multi_segment_path`
- `it_should_be_root_on_an_empty_path`
- `it_should_reject_an_absolute_path`
- `it_should_reject_a_path_containing_a_parent_traversal_segment`
- `it_should_reject_a_path_with_an_empty_segment`

**`application/http/directories/mod.rs`** (behavior tests, fake `DirectoryRepository`)
- `it_should_return_the_videos_roots_subdirectories_on_a_request_without_a_path` — 200, entries are the root's children
- `it_should_return_a_nested_directorys_subdirectories` — 200, only that level's children
- `it_should_return_an_empty_entry_list_on_a_directory_with_no_subdirectories` — 200, `entries: []`
- `it_should_report_the_videos_root_on_every_successful_listing` — `root` present on both a root listing and a nested one
- `it_should_return_bad_request_on_a_path_with_a_parent_traversal_segment` — 400, nothing disclosed
- `it_should_return_bad_request_on_an_absolute_path` — 400
- `it_should_return_not_found_on_a_path_the_repository_cannot_list` — 404
- `it_should_return_internal_server_error_on_a_repository_failure` — 500

**`infrastructure/repositories/filesystem_directory_repository.rs`** (real temp directories)
- `it_should_list_only_subdirectories_on_a_directory_containing_files_and_directories` — files omitted
- `it_should_omit_dot_prefixed_subdirectories`
- `it_should_return_entries_in_a_deterministic_order`
- `it_should_return_none_on_a_missing_directory`
- `it_should_return_none_on_a_path_that_is_a_regular_file`
- `it_should_return_none_on_a_symlink_resolving_outside_the_videos_root`
- `it_should_list_the_target_on_a_symlink_resolving_inside_the_videos_root`

**`serve.rs`** (real temp videos root)
- `it_should_create_the_default_storage_directories_on_an_empty_videos_root`
- `it_should_leave_existing_default_storage_directories_and_their_contents_untouched`
- `it_should_continue_starting_up_on_a_directory_creation_failure`

**`smoke-tests/`** (Playwright, real daemon)
- `it should create a playlist into a browsed parent folder` — destination preview matches, videos land there
- `it should create a playlist into a staged parent folder that did not exist` — preview names both new directories, both appear on disk after the first download
- `it should reach a sibling of the default parent via the breadcrumb` — the `/videos/kids/...` case, without leaving the dialog
- `it should descend into an existing directory when the create-folder step names one`
- `it should block submission when the destination is already used by another playlist`
- `it should reject a folder name containing a slash`
