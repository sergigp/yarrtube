# Design

## Files

- `.github/workflows/release.yml` — the `build` job computes `YARRTUBE_VERSION` (`inputs.tag`, else `edge-<short sha>` on `workflow_dispatch`, else `github.ref_name`) and passes it as a build arg.
- `Dockerfile` — `ARG YARRTUBE_VERSION` in the `build` stage, after `cargo chef cook`, right before `cargo build --release`, so changing it never invalidates the dependency layer.
- `src/infrastructure/shared/app_version.rs` — the compile-time `APP_VERSION` constant (build info, like `web_assets.rs`).
- `src/infrastructure/shared/mod.rs` — registers `app_version`.
- `src/application/http/mod.rs` — `RunningVersion` state (next to `VideosRoot`), added to `ApiServices`; `GET /version` route.
- `src/application/http/version/mod.rs` — `get_version` handler.
- `src/application/http/version/dto.rs` — `VersionResponse`.
- `src/serve.rs` — builds `RunningVersion(APP_VERSION.to_string())` in `api_services`.
- `web/src/api/types.ts` — `AppVersion` mirroring `VersionResponse`.
- `web/src/api/client.ts` — `fetchVersion`.
- `web/src/api/queries.ts` — `useVersion` (fetched once per page load) + query key.
- `web/src/components/VersionLabel.tsx` — muted `text-xs` version text; renders nothing unless loaded.
- `web/src/App.tsx` — renders `<VersionLabel />` right after the logo `<Link>`, as its sibling (the link's `aria-label` would hide it, and clicking it must not navigate).
- `web/src/AppShell.test.tsx` — routes `GET /api/version` in `renderShell`; version tests.
- `smoke-tests/tests/version.spec.js` — the header shows `dev` against a local build (route wiring).

No domain service: the version is build metadata, not business logic, so the handler reads it straight from state, as `VideosRoot` is read.

## Types & Signatures

```rust
// infrastructure/shared/app_version.rs
/// The release this binary was built for, stamped by the release build
/// (`YARRTUBE_VERSION`); `dev` when built without one. Cargo.toml's version
/// is deliberately not used: CI bumps only the git tag.
pub const APP_VERSION: &str = match option_env!("YARRTUBE_VERSION") {
    Some(version) => version,
    None => "dev",
};

// application/http/mod.rs
#[derive(Clone)]
pub struct RunningVersion(pub String);

pub struct ApiServices {
    // ...existing fields
    pub running_version: RunningVersion,
}

// application/http/version/dto.rs
#[derive(Debug, PartialEq, Serialize)]
pub struct VersionResponse {
    pub version: String,
}

// application/http/version/mod.rs
pub async fn get_version(
    State(RunningVersion(version)): State<RunningVersion>,
) -> Json<VersionResponse>;
```

```ts
// web/src/api/types.ts
export interface AppVersion { version: string }

// web/src/api/client.ts
export function fetchVersion(): Promise<AppVersion>

// web/src/api/queries.ts
export function useVersion(): UseQueryResult<AppVersion, Error> // staleTime: Infinity, no refetch on focus, no retry

// web/src/components/VersionLabel.tsx
export function VersionLabel(): JSX.Element | null
```

## Call Stack

Build:
```
release.yml build job
  -> docker/build-push-action (build-args: YARRTUBE_VERSION=<tag | edge-<sha>>)
    -> Dockerfile build stage: ARG YARRTUBE_VERSION
      -> cargo build --release  (option_env!("YARRTUBE_VERSION") -> APP_VERSION)
```

Request:
```
browser loads SPA
  -> AppShell -> VersionLabel -> useVersion()
    -> fetchVersion() -> GET /api/version
      -> get_version(State(RunningVersion(APP_VERSION)))
        -> Json(VersionResponse { version })
  -> VersionLabel renders "<version>" (nothing while pending or on error)
```

## Test Plan

1. Behaviour tests

   Rust (`application/http/version/mod.rs`):
   - `it_should_return_the_running_version` — `get_version(State(RunningVersion("v0.9.2")))` returns `Json(VersionResponse { version: "v0.9.2" })`.

   Web (`web/src/AppShell.test.tsx`):
   - `shows the running version next to the logo` — `GET /api/version` → `{ version: 'v0.9.2' }`; header shows exactly `v0.9.2`.
   - `stays on the current view when the version is clicked` — on `/tasks`, clicking `v0.9.2` leaves the tasks view rendered.
   - `shows no version while it is loading` — `pendingForever()`; no version text.
   - `shows no version and no error when the request fails` — `{ status: 500, error }`; no version text, no error text, the header still renders.

2. Infrastructure tests

   `infrastructure/shared/app_version.rs`:
   - `it_should_report_dev_without_a_build_version` — with `YARRTUBE_VERSION` unset at compile time (as in `cargo test`), `APP_VERSION == "dev"`.

   Smoke (`smoke-tests/tests/version.spec.js`, run against the local binary):
   - `shows the running version in the header` — the header shows `dev`.
