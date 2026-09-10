---
name: rust-architect
description: Architecture, naming, and testing conventions guidelines for architecting Rust codebases. Use when writing, reviewing, or refactoring Rust code: creating a new module or file, adding a repository/port/service/entity/value object, adding an HTTP handler or CLI command, deciding where a new .rs file goes, or writing/naming Rust tests.
---

# Philosophy

We try to follow a Domain Driven Design (DDD) approach with hexagonal architecture (aka ports and adapters) with some opinionated decisions. We try to follow clean architecture and we give a lot of importance to tests.

# Layers

We structure our code in three layers: application, domain, and infrastructure.

## Application Layer

This is the entry point to the application, the most common case will be http controllers but it can also be a CLI application, subscribers (message consuming from a queue system), etc. The main responsibility of this layer is to **VALIDATE** entry data and **TRANSFORM** it into Value Objects (VO). In the case of http this layer is the one responsible of parsing HTTP requests, validating the data and returning HTTP errors if not valid, transforming into VO and calling the domain layer, in the case that this the method needs to return a response it will transform the domain response into a response DTO and return it to the caller with the correct HTTP status code.

## Domain Layer

This is where the core **BUSINESS LOGIC** of the application lives. It is composed of entities, value objects, domain services, and domain events. The main responsibility of this layer is to implement the business rules and logic of the application and **ORCHESTRATE** calls to the ports in the infrastructure layer. We need to try to not depend on any external libraries or frameworks but some concessions could be done.

## Infrastructure Layer

This layer is responsible for implementing ports and **COMMUNICATING WITH EXTERNAL SYSTEMS**. Most of the time we will be coding repositories to access the database, but external clients to external APIs should be implemented here too. As a convention, even if this is not entirely correct bc this should go in the domain layer, we will place the trait of the repository here, near the implementation bc it's convenient when adding new methods to the repository. `repositories/` holds one aggregate's dedicated port implementation (a port injected into a single aggregate's domain service); `shared/` holds infra usable across aggregates, regardless of whether it's a port (e.g. the clock, domain events) — see the file structure below.
We must not hide behaviour in this layer, repositories should be as simple as possible and should behave like collections with methods like `find`, `find_by_id`, `save`, `delete`, etc. The domain layer should be the one that implements the business logic, rules and entity transformations, not the infrastructure layer. The infrastructure layer should be as simple as possible and should not contain any business logic like domain event publishing, it should only contain the operations on entities and optional monitoring. Write operations such as save, insert, update should have the entity as a parameter, not the properties. Delete could operate on the entity identifier instead of the entity itself and read methods such as find(id), find_all, etc should always return entities or collections of entities.

# File Structure and Naming and other opinions

We try to organize our domain code into modules with the aggregate name as module name. This is an example of how we would structure the code:

```
src/
  domain/
    <aggregate>/
      <value_object>.rs   # one file per value object, named after the type
      <entity>.rs         # named after the type, even if it repeats the folder name
      errors.rs           # every error type for this aggregate, together
      service.rs          # ALL orchestration logic for this aggregate's operations
  http/  (or grpc/, etc. — one per external interface)
    mod.rs                 # AppState + router wiring only
    error.rs               # shared response-mapping helpers
    <resource>/
      mod.rs                # handlers only
      dto.rs                # request/response wire types + From<Domain> conversions
  cli/                      # only if the project has a CLI
    mod.rs                  # Cli/Commands definitions (this is the CLI's "routing")
    <command>.rs             # one file per command's orchestration
  infrastructure/
    repositories/            # one aggregate's dedicated port implementation
      <implementation>_<port>.rs   # trait + its implementation, together
    client/                  # port-shaped adapters nothing in domain/ injects
      <port>.rs
    shared/                  # infra usable across aggregates (ports or not)
      <thing>.rs
```

- We prefer not generic names: not `entity.rs`, not `value_objects.rs`, not `ports.rs`. A file is named after the single type/concept it holds (`user.rs` holds `User`, `user_id.rs` holds `UserId`).
- `errors.rs` is the one deliberately generic name: every error type for an aggregate (validation errors, use-case error enums) lives together in one file, not scattered across the files that raise them.
- Every value object gets its own file. Don't bundle multiple value objects into one "value objects" file.
- A port is named after the concept it fronts, not the one method it happens to expose (`WidgetRepository`, not `WidgetLookup`, even if today it only has an `exists` method). "Repository" is used loosely for "adapter implementing a domain port," not strictly persistence.
- `infrastructure/repositories/` files are named `<implementation>_<port>.rs` (`sqlite_playlist_repository.rs` implements `PlaylistRepository` with SQLite, `youtube_video_downloader_repository.rs` implements `VideoDownloaderRepository` against YouTube/`yt-dlp`). The prefix signals which technology backs the port. `infrastructure/shared/` and `infrastructure/client/` files are named after the port/thing itself, not this convention, since they aren't per-aggregate repositories.
- A repository/port method either **reads** (returns `anyhow::Result<Entity>` / `anyhow::Result<Vec<Entity>>` / `anyhow::Result<Option<Entity>>`, an entity or collection, never a wrapper/outcome enum) or **writes** (returns `anyhow::Result<()>`, void on success). No custom infra error types (`RepositoryError`/`LookupError` and friends), infra failures are untyped `anyhow::Error`. A Repository always returns the entities that its name implies (`UserRepository` returns `User`).
- Business-meaningful outcomes that look like they belong in the repository (e.g. "was this newly created, or did it already exist?") are decided in the domain service, not returned by infra: call `find`, branch on `Some`/`None`, then call `insert`/`save`. This trades DB-level atomicity for keeping business logic out of infra, a known, accepted race window, not an oversight.
- State transitions live on the entity, never as behavior-named repository methods or as anemic domain models operated from domain service.

# Testing

We have mainly two types of tests: **behavior tests** and **infrastructure tests**. The first ones are the most important, they test the domain logic and they should be fast and isolated. The second ones are slower and they test the integration with external systems as real as possible. Our goal is to couple our tests as much as possible to behaviour instead of implementation, so we can refactor the code without breaking the tests.

## Behaviour Tests

This tests the domain logic and the validations at application level. We will place this tests in application (for example in http controllers or event subscribers) and the test will be the type of "I receive this HTTP request and I expect this response and these collateral effects". All of this will be using Fake implementations of the ports that the domain service uses. We will send HTTP requests and assert HTTP responses and final state of fake repositories. In the case of event subscribers we will send events and assert the final state of fake repositories. Very similar for Tasks, we will create tasks and assert the final state of fake repositories.

The fakes will be hand-written and will be as simple as possible, they will not use any mocking library. The fakes will be state-based, for example a fake repository backed by a `Mutex<Vec<Entity>>`.
Ideally we should not have tests in domain folder as all logic there is tested from application layer tests. There could be exceptions for very complex domain logic that is hard to test from application layer, but this should be the exception and not the rule.

Clock and Domain Event Publisher are treated like ports, so we will use fakes for them too. The fake clock will be a simple `Mutex<Instant>` and the fake domain event publisher will be a `Mutex<Vec<DomainEvent>>`.

## Infrastructure Tests

Infrastructure tests use the real dependency, colocated with the code. Some examples:

- Embedded engine (e.g. SQLite) -> a real in-memory instance, not a container, in-memory _is_ the real engine.
- Networked service (Postgres, Kafka, Redis...) -> testcontainers.
- Outbound HTTP dependency -> the real HTTP client against a fake/mock server, not a container.

Keep tests in `#[cfg(test)] mod tests` in the same file as the code they test by default. Only introduce a top-level `tests/` directory (which needs a `lib.rs`, turning the project into a library + thin binary) when a project independently justifies it — not just to match a template.

## Testing Opinions

- Test naming: `it_should_<expected outcome>_on_<condition>` — drop the `_on_...` part when there's no meaningful precondition beyond "given valid input" (`it_should_build_the_widget_url`). This naming focuses on behaviour instead of implementation.
- Tests should be as deterministic as possible. In the case of time we will use a fake clock that we can control. This will behave as a regular port.

# Other Opinions

- Code should be as functional as possible, we prefer mapping/folding/filtering over imperative loops, and immutable data over mutable data. We will use `Option` and `Result` types instead of nulls and exceptions. We will use `iterators` instead of `for` loops when possible. We will use `map`, `filter`, `fold`, etc. instead of imperative loops when possible.

# Deliberate deviations. Don't "fix" these

Some of these are deliberate deviations from DDD/hexagonal orthodoxy, for convenience or to avoid overengineering:

- Port traits live in `infrastructure/`, not `domain/` (dependency direction inverted, traded for editing convenience).
- Repositories have no typed error enums, everything is `anyhow::Error` until the domain service gives it meaning.
- An idempotent "create" use case is two non-atomic port calls (`find` then `insert`), not one atomic upsert.
- Orchestration lives in `domain/<aggregate>/service.rs`, not a separate `application/` layer.

If code looks like it violates textbook architecture in one of these ways, it's probably intentional. Ask before changing it.
