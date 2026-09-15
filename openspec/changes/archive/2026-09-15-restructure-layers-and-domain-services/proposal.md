## Why

Domain services and the top-level source layout have grown past what the project's own layering rules describe: `playlist/service.rs` and `video/service.rs` each bundle several unrelated operations (and every dependency any of them needs) behind one struct, private helpers land wherever was convenient when they were written, and `http/`, `subscribers/`, `tasks/`, and `cli/` sit as flat siblings even though the `rust-architect` skill already describes them conceptually as one "application" entry-point layer. New domain work that will change the shape of the app significantly is coming next; this is the moment to realign the structure and the skill that documents it before that work lands on top of the current shape.

## What Changes

- **Fn ordering**: within any `impl` block (and among free functions in a file), order becomes: `new` (if it exists), then every `pub` method or trait-impl method, then private/helper methods — no exceptions, including the SQLite repositories' `row_to_*` mapping helpers, which currently sit before the trait-impl methods they support.
- **Split `domain/playlist/service.rs`** into per-operation services under a new `domain/services/` folder: `PlaylistCreator` (`create`, `create_custom`), `PlaylistDeleter` (`delete`), `PlaylistSearcher` (`search`, `search_all`) — each injected only with the ports its own methods use.
- **Split `domain/video/service.rs`** the same way: `VideoReconciler` (`reconcile`, `force_reconcile`, plus the private reconciliation helpers), `VideoDownloader` (`download`), `VideoFileDeleter` (`delete_video_file`, `delete_playlist_video_files`), `CustomPlaylistVideoAdder` (`add`), `CustomPlaylistVideoRemover` (`remove`), `VideoSearcher` (`find`, `list`).
- **New `application/` top-level folder**: `http/`, `subscribers/`, and `tasks/` move under it (`application/http`, `application/subscribers`, `application/tasks`), matching the skill's existing "Application Layer" description, which already names subscribers as an application-layer concern even though the file structure never reflected it.
- **`cli/` split by actual responsibility**: the CLI parsing itself (`Cli`/`Commands`) and the thin `update-ytdlp` command dispatcher move to `application/cli/`; the GitHub-release lookup, binary download, and atomic install logic in `cli/ytdlp_update.rs` moves into `infrastructure/client/ytdlp_updater.rs`, removing the current `infrastructure -> cli` dependency inversion (the real `YtdlpUpdater` port implementation currently calls back into `cli::ytdlp_update::update`).
- **Skill update** (`.claude/skills/rust-architect/SKILL.md`): document the `application/` layer and `domain/services/` folder in the File Structure section, state explicitly that domain services are call-agnostic and that adapting an external trigger (HTTP, CLI, subscriber, task) into a domain call is the application layer's responsibility, and document the fn-ordering rule.
- **BREAKING (internal only)**: `PlaylistService` and `VideoService` are removed as public types; every construction site and consumer (HTTP handlers, subscribers, task handlers, `serve.rs`) is updated to depend on the specific new service(s) it actually needs.

## Capabilities

No capability specs are created or modified — this is a pure internal restructuring (module layout, type/file naming, dependency wiring, one documentation skill) with no change in externally observable behavior. `.openspec.yaml` sets `skip_specs: true` accordingly.

## Impact

- **Domain**: `domain/playlist/service.rs` and `domain/video/service.rs` are deleted; their logic moves into 3 + 6 new files under `domain/services/`. `domain/mod.rs` gains `pub mod services;`.
- **Application (new)**: `src/http/`, `src/subscribers/`, `src/tasks/`, and the CLI-parsing part of `src/cli/` move under `src/application/`.
- **Infrastructure**: `infrastructure/client/ytdlp_updater.rs` absorbs the GitHub/filesystem logic currently in `cli/ytdlp_update.rs`; three SQLite repositories and two other infrastructure files get their private helpers reordered.
- **Call sites**: every place that builds or holds a `PlaylistService`/`VideoService` (`serve.rs`, HTTP handlers and their `AppState`, subscribers, task handlers, and their tests) is updated to the narrower replacement service(s).
- **`main.rs`**: module declarations updated (`mod application;` replacing `mod http; mod subscribers; mod tasks; mod cli;`).
- **Tests**: existing behavior tests move with their code and are updated to construct the narrower services; no test behavior/assertions change.
- **Docs**: `.claude/skills/rust-architect/SKILL.md` updated.
